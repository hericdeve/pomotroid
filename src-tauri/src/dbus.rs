//! D-Bus IPC service for Pomotroid desktop integration (`org.pomotroid.Pomodoro`).
//!
//! Exposes timer controls, session tags with database-backed autocompletion,
//! round goal adjustments, and window focus controls on the user session bus.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};
use tauri::{AppHandle, Emitter, Manager};
use zbus::{interface, connection::Builder, zvariant::Value};

use crate::db::{queries, DbState};
use crate::settings;
use crate::timer::{TimerController, TimerSnapshot};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PendingTags {
    pub subject: String,
    pub subject_topic: String,
    pub study_type: String,
    pub notes: String,
}

pub struct DbusState {
    pub pending_tags: Arc<Mutex<PendingTags>>,
    pub goal_rounds: Arc<AtomicU32>,
    pub connection: tokio::sync::Mutex<Option<zbus::Connection>>,
}

impl DbusState {
    pub fn new(default_goal: u32) -> Self {
        Self {
            pending_tags: Arc::new(Mutex::new(PendingTags::default())),
            goal_rounds: Arc::new(AtomicU32::new(default_goal)),
            connection: tokio::sync::Mutex::new(None),
        }
    }
}

pub struct PomotroidDbus {
    app: AppHandle,
    pending_tags: Arc<Mutex<PendingTags>>,
    goal_rounds: Arc<AtomicU32>,
}

#[interface(name = "org.pomotroid.Pomodoro")]
impl PomotroidDbus {
    /// Toggle timer: start, pause, or resume.
    async fn timer_toggle(&self) {
        log::info!("[dbus] TimerToggle called");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            timer.toggle();
        }
    }

    /// Explicitly start/resume timer.
    async fn timer_start(&self) {
        log::info!("[dbus] TimerStart called");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            let snap = timer.get_snapshot();
            if !snap.is_running {
                timer.toggle();
            }
        }
    }

    /// Explicitly pause timer.
    async fn timer_pause(&self) {
        log::info!("[dbus] TimerPause called");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            let snap = timer.get_snapshot();
            if snap.is_running {
                timer.toggle();
            }
        }
    }

    /// Reset current round / sequence.
    async fn timer_reset(&self) {
        log::info!("[dbus] TimerReset called");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            timer.reset();
        }
    }

    /// Skip current round to next phase.
    async fn timer_skip(&self) {
        log::info!("[dbus] TimerSkip called");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            timer.skip();
        }
    }

    /// Restart current round from 0 elapsed without advancing sequence ("back to start").
    async fn timer_restart_round(&self) {
        log::info!("[dbus] TimerRestartRound called");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            timer.restart_round();
        }
    }

    /// Return complete snapshot of timer and session metadata as a dictionary.
    async fn get_snapshot(&self) -> HashMap<String, Value<'static>> {
        let mut map = HashMap::new();
        let snap = self.app.try_state::<TimerController>()
            .map(|t| t.get_snapshot())
            .unwrap_or_else(|| TimerSnapshot {
                round_type: "work".to_string(),
                previous_round_type: String::new(),
                elapsed_secs: 0,
                total_secs: 1500,
                is_running: false,
                is_paused: false,
                work_round_number: 1,
                work_rounds_total: 4,
                session_work_count: 0,
                active_session_id: None,
                last_completed_session_id: None,
                active_study_session_id: None,
            });

        let tags = self.pending_tags.lock().unwrap().clone();
        let goal = self.goal_rounds.load(Ordering::Relaxed);

        map.insert("round_type".into(), Value::from(snap.round_type));
        map.insert("elapsed_secs".into(), Value::from(snap.elapsed_secs));
        map.insert("total_secs".into(), Value::from(snap.total_secs));
        map.insert("is_running".into(), Value::from(snap.is_running));
        map.insert("is_paused".into(), Value::from(snap.is_paused));
        map.insert("work_round_number".into(), Value::from(snap.work_round_number));
        map.insert("work_rounds_total".into(), Value::from(snap.work_rounds_total));
        map.insert("session_work_count".into(), Value::from(snap.session_work_count));
        map.insert("goal_rounds".into(), Value::from(goal));
        map.insert("subject".into(), Value::from(tags.subject));
        map.insert("subject_topic".into(), Value::from(tags.subject_topic));
        map.insert("study_type".into(), Value::from(tags.study_type));
        map.insert("notes".into(), Value::from(tags.notes));

        map
    }

    /// Set session tags and persist them to the database if a study session / round is active.
    async fn set_tags(&self, subject: String, subject_topic: String, study_type: String, notes: String) {
        log::info!("[dbus] SetTags subject='{subject}', topic='{subject_topic}', type='{study_type}'");
        {
            let mut tags = self.pending_tags.lock().unwrap();
            tags.subject = subject.clone();
            tags.subject_topic = subject_topic.clone();
            tags.study_type = study_type.clone();
            tags.notes = notes.clone();
        }

        // Sync to active study session and round in SQLite if running
        if let Some(timer) = self.app.try_state::<TimerController>() {
            let snap = timer.get_snapshot();
            if let Some(db) = self.app.try_state::<DbState>() {
                if let Ok(conn) = db.lock() {
                    if let Some(study_id) = snap.active_study_session_id {
                        let _ = queries::update_study_session(&conn, study_id, queries::UpdateStudySessionPayload {
                            subject: if subject.is_empty() { None } else { Some(subject.clone()) },
                            subject_topic: if subject_topic.is_empty() { None } else { Some(subject_topic.clone()) },
                            study_type: if study_type.is_empty() { None } else { Some(study_type.clone()) },
                            notes: if notes.is_empty() { None } else { Some(notes.clone()) },
                            goal_rounds: None,
                        });
                    }
                    if let Some(round_id) = snap.active_session_id {
                        let _ = queries::update_session(&conn, round_id, queries::UpdateSessionPayload {
                            subject: if subject.is_empty() { None } else { Some(subject.clone()) },
                            subject_topic: if subject_topic.is_empty() { None } else { Some(subject_topic.clone()) },
                            study_type: if study_type.is_empty() { None } else { Some(study_type.clone()) },
                            notes: if notes.is_empty() { None } else { Some(notes.clone()) },
                            duration_secs: None,
                            exclude_from_stats: None,
                            started_at: None,
                        });
                    }
                }
            }
        }

        // Notify Svelte frontend
        let _ = self.app.emit("tags:changed", serde_json::json!({
            "subject": subject,
            "subject_topic": subject_topic,
            "study_type": study_type,
            "notes": notes,
        }));

        // Emit D-Bus signal
        if let Some(dbus_state) = self.app.try_state::<Arc<DbusState>>() {
            if let Some(ref conn) = *dbus_state.connection.lock().await {
                let _ = conn.emit_signal(
                    None::<()>,
                    "/org/pomotroid/Pomodoro",
                    "org.pomotroid.Pomodoro",
                    "TagsChanged",
                    &(subject, subject_topic, study_type, notes),
                ).await;
            }
        }
    }

    /// Set round goal for the active session.
    async fn set_goal_rounds(&self, goal: u32) {
        log::info!("[dbus] SetGoalRounds: {goal}");
        self.goal_rounds.store(goal, Ordering::Relaxed);

        if let Some(db) = self.app.try_state::<DbState>() {
            if let Ok(conn) = db.lock() {
                let _ = settings::save_setting(&conn, "session_goal_rounds", &goal.to_string());
                if let Some(timer) = self.app.try_state::<TimerController>() {
                    let snap = timer.get_snapshot();
                    if let Some(study_id) = snap.active_study_session_id {
                        let _ = queries::update_study_session(&conn, study_id, queries::UpdateStudySessionPayload {
                            subject: None,
                            subject_topic: None,
                            study_type: None,
                            notes: None,
                            goal_rounds: Some(goal),
                        });
                    }
                }
            }
        }

        let _ = self.app.emit("goal:changed", goal);

        if let Some(dbus_state) = self.app.try_state::<Arc<DbusState>>() {
            if let Some(ref conn) = *dbus_state.connection.lock().await {
                let _ = conn.emit_signal(
                    None::<()>,
                    "/org/pomotroid/Pomodoro",
                    "org.pomotroid.Pomodoro",
                    "GoalChanged",
                    &(goal,),
                ).await;
            }
        }
    }

    /// Query distinct subjects from the Pomotroid SQLite database.
    async fn get_subjects(&self) -> Vec<String> {
        if let Some(db) = self.app.try_state::<DbState>() {
            if let Ok(conn) = db.lock() {
                return queries::get_distinct_subjects(&conn).unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Query distinct topics for a given subject (or all distinct topics if subject is empty).
    async fn get_topics(&self, subject: String) -> Vec<String> {
        if let Some(db) = self.app.try_state::<DbState>() {
            if let Ok(conn) = db.lock() {
                let s = if subject.trim().is_empty() { None } else { Some(subject) };
                return queries::get_distinct_topics(&conn, s.as_deref()).unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Query distinct study types from the Pomotroid SQLite database.
    async fn get_study_types(&self) -> Vec<String> {
        if let Some(db) = self.app.try_state::<DbState>() {
            if let Ok(conn) = db.lock() {
                return queries::get_distinct_study_types(&conn).unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Bring the main Pomotroid window to the foreground.
    async fn open_main_window(&self) {
        log::info!("[dbus] OpenMainWindow called");
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    /// Open or focus the Pomotroid statistics window.
    async fn open_stats_window(&self) {
        log::info!("[dbus] OpenStatsWindow called");
        if let Some(existing) = self.app.get_webview_window("stats") {
            let _ = existing.show();
            let _ = existing.set_focus();
            return;
        }

        let is_mac = cfg!(target_os = "macos");
        let win = tauri::WebviewWindowBuilder::new(&self.app, "stats", tauri::WebviewUrl::App("/stats".into()))
            .title("Pomotroid — Statistics")
            .inner_size(840.0, 520.0)
            .min_inner_size(600.0, 400.0)
            .decorations(is_mac)
            .resizable(true)
            .build();

        if let Ok(w) = win {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

/// Initialize and start the D-Bus service on the session bus.
pub async fn start(app: AppHandle, dbus_state: Arc<DbusState>) {
    let pomotroid_dbus = PomotroidDbus {
        app: app.clone(),
        pending_tags: Arc::clone(&dbus_state.pending_tags),
        goal_rounds: Arc::clone(&dbus_state.goal_rounds),
    };

    match Builder::session() {
        Ok(builder) => {
            match builder
                .name("org.pomotroid.Pomodoro")
                .and_then(|b| b.serve_at("/org/pomotroid/Pomodoro", pomotroid_dbus))
            {
                Ok(b) => match b.build().await {
                    Ok(conn) => {
                        log::info!("[dbus] org.pomotroid.Pomodoro service registered on session bus");
                        *dbus_state.connection.lock().await = Some(conn);
                    }
                    Err(e) => {
                        log::warn!("[dbus] failed to build D-Bus connection: {e}");
                    }
                },
                Err(e) => {
                    log::warn!("[dbus] failed to configure D-Bus service: {e}");
                }
            }
        }
        Err(e) => {
            log::warn!("[dbus] failed to connect to session bus: {e}");
        }
    }
}

/// Broadcast a tick signal over D-Bus.
pub async fn broadcast_tick(state: &Arc<DbusState>, elapsed_secs: u32, total_secs: u32) {
    if let Some(ref conn) = *state.connection.lock().await {
        let _ = conn.emit_signal(
            None::<()>,
            "/org/pomotroid/Pomodoro",
            "org.pomotroid.Pomodoro",
            "Tick",
            &(elapsed_secs, total_secs),
        ).await;
    }
}

/// Broadcast a state change signal over D-Bus.
pub async fn broadcast_state_changed(
    state: &Arc<DbusState>,
    snap: &TimerSnapshot,
) {
    if let Some(ref conn) = *state.connection.lock().await {
        let mut map = HashMap::new();
        let tags = state.pending_tags.lock().unwrap().clone();
        let goal = state.goal_rounds.load(Ordering::Relaxed);

        map.insert("round_type".to_string(), Value::from(snap.round_type.clone()));
        map.insert("elapsed_secs".to_string(), Value::from(snap.elapsed_secs));
        map.insert("total_secs".to_string(), Value::from(snap.total_secs));
        map.insert("is_running".to_string(), Value::from(snap.is_running));
        map.insert("is_paused".to_string(), Value::from(snap.is_paused));
        map.insert("work_round_number".to_string(), Value::from(snap.work_round_number));
        map.insert("work_rounds_total".to_string(), Value::from(snap.work_rounds_total));
        map.insert("session_work_count".to_string(), Value::from(snap.session_work_count));
        map.insert("goal_rounds".to_string(), Value::from(goal));
        map.insert("subject".to_string(), Value::from(tags.subject));
        map.insert("subject_topic".to_string(), Value::from(tags.subject_topic));
        map.insert("study_type".to_string(), Value::from(tags.study_type));
        map.insert("notes".to_string(), Value::from(tags.notes));

        let _ = conn.emit_signal(
            None::<()>,
            "/org/pomotroid/Pomodoro",
            "org.pomotroid.Pomodoro",
            "StateChanged",
            &(map,),
        ).await;
    }
}
