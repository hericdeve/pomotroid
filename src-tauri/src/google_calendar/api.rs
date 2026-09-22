use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::google_calendar::GoogleCalendarItem;

const CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";

#[derive(Deserialize)]
struct CalendarListResponse {
    #[serde(default)]
    items: Vec<CalendarListItem>,
}

#[derive(Deserialize)]
struct CalendarListItem {
    id: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(rename = "summaryOverride")]
    summary_override: Option<String>,
    description: Option<String>,
    primary: Option<bool>,
    #[serde(rename = "backgroundColor")]
    background_color: Option<String>,
    #[serde(rename = "foregroundColor")]
    foreground_color: Option<String>,
}

#[derive(Deserialize)]
struct EventListResponse {
    #[serde(default)]
    items: Vec<GoogleEventItem>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GoogleEventItem {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "htmlLink")]
    pub html_link: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "recurringEventId")]
    pub recurring_event_id: Option<String>,
    pub start: Option<EventDateTime>,
    pub end: Option<EventDateTime>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct EventDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
}

pub async fn fetch_calendars(access_token: &str) -> Result<Vec<GoogleCalendarItem>, String> {
    let client = Client::new();
    let url = format!("{CALENDAR_API_BASE}/users/me/calendarList?maxResults=250");

    let res = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Calendar list request failed: {e}"))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        log::error!("[gcal] fetch_calendars failed (HTTP {status}): {body}");
        return Err(format!("Failed to list calendars (HTTP {status}): {body}"));
    }

    let parsed: CalendarListResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse calendar list response: {e}"))?;

    let items = parsed
        .items
        .into_iter()
        .map(|item| {
            let summary = item
                .summary_override
                .or(item.summary)
                .unwrap_or_else(|| "Untitled Calendar".into());
            GoogleCalendarItem {
                id: item.id,
                summary,
                description: item.description,
                primary: item.primary.unwrap_or(false),
                background_color: item.background_color.unwrap_or_else(|| "#4285F4".into()),
                foreground_color: item.foreground_color.unwrap_or_else(|| "#FFFFFF".into()),
                is_visible: false,
                is_synced: false,
            }
        })
        .collect();

    Ok(items)
}

pub async fn fetch_events(
    access_token: &str,
    calendar_id: &str,
    time_min: &str,
    time_max: &str,
) -> Result<Vec<GoogleEventItem>, String> {
    let client = Client::new();
    let encoded_cal_id = urlencoding::encode(calendar_id);
    let url = format!(
        "{CALENDAR_API_BASE}/calendars/{encoded_cal_id}/events?singleEvents=true&timeMin={}&timeMax={}&orderBy=startTime&maxResults=250",
        urlencoding::encode(time_min),
        urlencoding::encode(time_max),
    );

    let res = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch events for {calendar_id}: {e}"))?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Event fetch error ({calendar_id}): {body}"));
    }

    let parsed: EventListResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse events: {e}"))?;

    // Filter out cancelled events
    let active_events = parsed
        .items
        .into_iter()
        .filter(|e| e.status.as_deref() != Some("cancelled"))
        .collect();

    Ok(active_events)
}

pub fn get_local_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string())
}

pub async fn create_event(
    access_token: &str,
    calendar_id: &str,
    summary: &str,
    start_iso: &str,
    end_iso: &str,
    time_zone: Option<&str>,
) -> Result<String, String> {
    let client = Client::new();
    let encoded_cal_id = urlencoding::encode(calendar_id);
    let url = format!("{CALENDAR_API_BASE}/calendars/{encoded_cal_id}/events");

    let tz = time_zone
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(get_local_timezone);

    let payload = json!({
        "summary": summary,
        "start": {
            "dateTime": start_iso,
            "timeZone": tz
        },
        "end": {
            "dateTime": end_iso,
            "timeZone": tz
        },
        "recurrence": [
            "RRULE:FREQ=WEEKLY"
        ]
    });

    let res = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Create event request failed: {e}"))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        log::error!("[gcal] create_event failed (HTTP {status}): {body}");
        return Err(format!("Failed to create event in Google Calendar (HTTP {status}): {body}"));
    }

    let created: GoogleEventItem = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse created event: {e}"))?;

    Ok(created.id)
}

pub async fn update_event(
    access_token: &str,
    calendar_id: &str,
    event_id: &str,
    summary: &str,
    start_iso: &str,
    end_iso: &str,
    time_zone: Option<&str>,
) -> Result<(), String> {
    let client = Client::new();
    let encoded_cal_id = urlencoding::encode(calendar_id);
    let encoded_event_id = urlencoding::encode(event_id);
    let url = format!("{CALENDAR_API_BASE}/calendars/{encoded_cal_id}/events/{encoded_event_id}");

    let tz = time_zone
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(get_local_timezone);

    let payload = json!({
        "summary": summary,
        "start": {
            "dateTime": start_iso,
            "timeZone": tz
        },
        "end": {
            "dateTime": end_iso,
            "timeZone": tz
        }
    });

    let res = client
        .patch(&url)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Update event request failed: {e}"))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        log::error!("[gcal] update_event failed (HTTP {status}): {body}");
        return Err(format!("Failed to update event in Google Calendar (HTTP {status}): {body}"));
    }

    Ok(())
}


pub async fn delete_event(
    access_token: &str,
    calendar_id: &str,
    event_id: &str,
) -> Result<(), String> {
    let client = Client::new();
    let encoded_cal_id = urlencoding::encode(calendar_id);
    let encoded_event_id = urlencoding::encode(event_id);
    let url = format!("{CALENDAR_API_BASE}/calendars/{encoded_cal_id}/events/{encoded_event_id}");

    let res = client
        .delete(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Delete event request failed: {e}"))?;

    // 404 or 410 Gone is considered successfully deleted
    if !res.status().is_success() && res.status() != 404 && res.status() != 410 {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Failed to delete event in Google Calendar: {body}"));
    }

    Ok(())
}
