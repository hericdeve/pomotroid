use std::sync::Arc;
use std::time::Duration;
use axum::{
    extract::{Query, State},
    response::Html,
    routing::get,
    Router,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::{oneshot, Mutex};
use crate::db::{queries, DbState};
use crate::db::queries::GoogleAuthRow;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const OAUTH_SCOPES: &str = "https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/userinfo.email";

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct UserInfoResponse {
    email: Option<String>,
}

fn generate_random_string(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    for b in &mut bytes {
        *b = (uuid::Uuid::new_v4().as_u128() & 0xFF) as u8;
    }
    URL_SAFE_NO_PAD.encode(&bytes)
}

pub async fn start_browser_oauth(db: &DbState) -> Result<String, String> {
    let (client_id, client_secret) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let auth_record = queries::get_google_auth(&conn)
            .map_err(|e| format!("Failed to read auth settings: {e}"))?;

        let cid = auth_record
            .as_ref()
            .map(|a| a.client_id.clone())
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| "Google Client ID is not configured. Please enter your OAuth Client ID in settings.".to_string())?;

        let csec = auth_record.and_then(|a| a.client_secret).filter(|s| !s.trim().is_empty());
        (cid, csec)
    };

    let code_verifier = generate_random_string(32);
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    let state = uuid::Uuid::new_v4().to_string();

    // Bind loopback listener
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:54321").await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind loopback port: {e}"))?,
    };

    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local port: {e}"))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx_state = Arc::new(Mutex::new(Some((tx, state.clone()))));

    let router = Router::new()
        .route("/callback", get(move |Query(params): Query<CallbackQuery>, State(state_holder): State<Arc<Mutex<Option<(oneshot::Sender<Result<String, String>>, String)>>>>| async move {
            let mut guard = state_holder.lock().await;
            if let Some((sender, expected_state)) = guard.take() {
                if let Some(err) = params.error {
                    let _ = sender.send(Err(format!("Google error: {err}")));
                    return Html(r#"<!DOCTYPE html><html><body style="background:#18181b;color:#f87171;font-family:sans-serif;text-align:center;padding:50px;"><h2>Authentication Failed</h2><p>You can close this tab and return to Pomotroid.</p></body></html>"#);
                }
                if params.state.as_deref() != Some(&expected_state) {
                    let _ = sender.send(Err("Invalid CSRF state received".into()));
                    return Html(r#"<!DOCTYPE html><html><body style="background:#18181b;color:#f87171;font-family:sans-serif;text-align:center;padding:50px;"><h2>Authentication Error</h2><p>State mismatch. You can close this tab.</p></body></html>"#);
                }
                if let Some(code) = params.code {
                    let _ = sender.send(Ok(code));
                    return Html(r#"<!DOCTYPE html><html><body style="background:#18181b;color:#f4f4f5;font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;"><div style="background:#27272a;padding:30px 40px;border-radius:12px;box-shadow:0 4px 20px rgba(0,0,0,0.5);text-align:center;max-width:400px;"><h2 style="color:#22c55e;margin-top:0;">Authentication Successful!</h2><p style="color:#a1a1aa;font-size:15px;line-height:1.5;">You've connected Google Calendar to Pomotroid.<br>You may now close this browser tab and return to the app.</p></div></body></html>"#);
                }
                let _ = sender.send(Err("No authorization code found in response".into()));
            }
            Html(r#"<!DOCTYPE html><html><body style="background:#18181b;color:#f4f4f5;font-family:sans-serif;text-align:center;padding:50px;"><h2>Pomotroid</h2><p>Request processed. You can close this window.</p></body></html>"#)
        }))
        .with_state(tx_state);

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
        GOOGLE_AUTH_URL,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(OAUTH_SCOPES),
        urlencoding::encode(&code_challenge),
        urlencoding::encode(&state),
    );

    // Open URL in default browser
    if let Err(e) = tauri_plugin_opener::open_url(&auth_url, None::<&str>) {
        log::warn!("[gcal] failed to open browser via plugin: {e}");
    }

    // Serve callback with a 3-minute timeout
    let serve_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let code = match tokio::time::timeout(Duration::from_secs(180), rx).await {
        Ok(Ok(res)) => res?,
        Ok(Err(_)) => return Err("OAuth callback channel closed unexpectedly".into()),
        Err(_) => return Err("Authentication timed out waiting for browser sign-in".into()),
    };

    serve_handle.abort();

    // Exchange authorization code for tokens
    let http_client = reqwest::Client::new();
    let mut body = format!(
        "code={}&client_id={}&redirect_uri={}&grant_type=authorization_code&code_verifier={}",
        urlencoding::encode(&code),
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&code_verifier),
    );
    if let Some(ref cs) = client_secret {
        body.push_str(&format!("&client_secret={}", urlencoding::encode(cs)));
    }

    let token_res = http_client
        .post(GOOGLE_TOKEN_URL)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {e}"))?;

    if !token_res.status().is_success() {
        let err_body = token_res.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {err_body}"));
    }

    let tokens: TokenResponse = token_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    // Fetch user email
    let user_info_res = http_client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(&tokens.access_token)
        .send()
        .await;

    let email = match user_info_res {
        Ok(res) if res.status().is_success() => res
            .json::<UserInfoResponse>()
            .await
            .ok()
            .and_then(|u| u.email),
        _ => None,
    };

    let expires_at = queries::unix_now() + tokens.expires_in;

    let auth_to_save = GoogleAuthRow {
        client_id,
        client_secret,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at,
        email: email.clone(),
    };

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        queries::save_google_auth(&conn, &auth_to_save)
            .map_err(|e| format!("Failed to save auth to database: {e}"))?;
    }

    Ok(email.unwrap_or_else(|| "Connected".to_string()))
}

pub async fn ensure_valid_token(db: &DbState) -> Result<String, String> {
    let auth = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        queries::get_google_auth(&conn)
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| "Not signed in to Google Calendar".to_string())?
    };

    let now = queries::unix_now();
    // If token has at least 60 seconds of validity left, return it
    if auth.expires_at > now + 60 && !auth.access_token.is_empty() {
        return Ok(auth.access_token);
    }

    let refresh_token = auth
        .refresh_token
        .filter(|r| !r.trim().is_empty())
        .ok_or_else(|| "No refresh token available. Please sign in again.".to_string())?;

    let client = reqwest::Client::new();
    let mut body = format!(
        "client_id={}&refresh_token={}&grant_type=refresh_token",
        urlencoding::encode(&auth.client_id),
        urlencoding::encode(&refresh_token),
    );
    if let Some(ref cs) = auth.client_secret {
        body.push_str(&format!("&client_secret={}", urlencoding::encode(cs)));
    }

    let res = client
        .post(GOOGLE_TOKEN_URL)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {e}"))?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Token refresh failed: {body}"));
    }

    let tokens: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse refreshed tokens: {e}"))?;

    let expires_at = now + tokens.expires_in;

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        queries::update_google_tokens(
            &conn,
            &tokens.access_token,
            tokens.refresh_token.as_deref(),
            expires_at,
        )
        .map_err(|e| format!("Failed to update tokens in database: {e}"))?;
    }

    Ok(tokens.access_token)
}
