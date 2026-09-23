//! D-Bus IPC service for Pomotroid desktop integration (`org.pomotroid.Pomodoro`).
//!
//! Exposes timer controls, session tags with database-backed autocompletion,
//! round goal adjustments, and window focus controls on the user session bus.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};
use tauri::{AppHandle, Emitter, Manager};
use zbus::{interface, connection::Builder, zvariant::Value, fdo};
use chrono::Datelike;

use crate::commands::DetailedStats;
use crate::db::{queries, DbState};
use crate::settings;
use crate::themes;
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

fn db_err(e: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(e.to_string())
}

fn json_err(e: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(format!("Serialization error: {e}"))
}

impl PomotroidDbus {
    fn get_db(&self) -> fdo::Result<DbState> {
        let db = self.app.try_state::<DbState>()
            .ok_or_else(|| fdo::Error::Failed("Database state not initialized".into()))?;
        Ok(db.inner().clone())
    }

    async fn emit_dbus_signal<B: serde::Serialize + zbus::zvariant::Type>(&self, signal_name: &str, body: &B) {
        if let Some(dbus_state) = self.app.try_state::<Arc<DbusState>>() {
            if let Some(ref conn) = *dbus_state.connection.lock().await {
                let _ = conn.emit_signal(
                    None::<()>,
                    "/org/pomotroid/Pomodoro",
                    "org.pomotroid.Pomodoro",
                    signal_name,
                    body,
                ).await;
            }
        }
    }

    async fn broadcast_events_changed(&self) {
        self.emit_dbus_signal("EventsChanged", &()).await;
        let _ = self.app.emit("events:changed", ());
    }

    async fn broadcast_subjects_changed(&self) {
        self.emit_dbus_signal("SubjectsChanged", &()).await;
        let _ = self.app.emit("subjects:changed", ());
    }

    async fn broadcast_schedule_changed(&self) {
        self.emit_dbus_signal("ScheduleChanged", &()).await;
        let _ = self.app.emit("schedule:changed", ());
    }

    async fn broadcast_theme_changed(&self, theme_name: &str) {
        self.emit_dbus_signal("ThemeChanged", &(theme_name,)).await;
    }

    async fn broadcast_setting_changed(&self, key: &str, value: &str) {
        self.emit_dbus_signal("SettingChanged", &(key, value)).await;
    }
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

    /// Add completed work rounds to active study session without running timer.
    async fn timer_add_completed_rounds(&self, count: u32) {
        log::info!("[dbus] TimerAddCompletedRounds: {count}");
        if let Some(timer) = self.app.try_state::<TimerController>() {
            timer.add_completed_rounds(&self.app, count);
        }
    }

    /// Return pending study tags for next round.
    async fn get_pending_tags(&self) -> HashMap<String, Value<'static>> {
        let tags = self.pending_tags.lock().unwrap().clone();
        let mut map = HashMap::new();
        map.insert("subject".into(), Value::from(tags.subject));
        map.insert("subject_topic".into(), Value::from(tags.subject_topic));
        map.insert("study_type".into(), Value::from(tags.study_type));
        map.insert("notes".into(), Value::from(tags.notes));
        map
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
                    if (snap.is_running || snap.is_paused) && snap.round_type == "work" {
                        if let Some(round_id) = snap.active_session_id {
                            let _ = queries::update_session(&conn, round_id, queries::UpdateSessionPayload {
                                subject: if subject.is_empty() { None } else { Some(subject.clone()) },
                                subject_topic: if subject_topic.is_empty() { None } else { Some(subject_topic.clone()) },
                                study_type: if study_type.is_empty() { None } else { Some(study_type.clone()) },
                                notes: if notes.is_empty() { None } else { Some(notes.clone()) },
                                duration_secs: None,
                                exclude_from_stats: None,
                                started_at: None,
                                completed: None,
                                is_half_session: None,
                                round_type: None,
                            });
                        }
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

    // -----------------------------------------------------------------------
    // Academic Events (Assignments & Exams)
    // -----------------------------------------------------------------------

    /// Fetch all academic events serialized as a JSON string.
    async fn get_subject_events(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let events = queries::subject_events_get_all(&conn).map_err(db_err)?;
        serde_json::to_string(&events).map_err(json_err)
    }

    /// Fetch upcoming non-completed academic events (up to `limit`, or all if `limit` == 0) as JSON.
    async fn get_upcoming_events(&self, limit: u32) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let mut events = queries::subject_events_get_all(&conn).map_err(db_err)?;
        events.retain(|e| !e.is_completed);
        events.sort_by(|a, b| {
            a.event_date.cmp(&b.event_date)
                .then_with(|| a.event_time.cmp(&b.event_time))
        });
        let take_count = if limit == 0 { events.len() } else { (limit as usize).min(events.len()) };
        let upcoming: Vec<_> = events.into_iter().take(take_count).collect();
        serde_json::to_string(&upcoming).map_err(json_err)
    }

    /// Create a new academic event directly over D-Bus.
    async fn subject_event_create(
        &self,
        subject: String,
        name: String,
        event_type: String,
        event_date: String,
        event_time: String,
        end_date: String,
        end_time: String,
        notes: String,
        is_all_day: bool,
    ) -> fdo::Result<i64> {
        let db = self.get_db()?;
        let payload = {
            let conn = db.lock().map_err(db_err)?;
            let events_cal = queries::get_google_events_calendar(&conn).ok().flatten();
            let (calendar_type, google_calendar_id) = if let Some(cal) = events_cal {
                (Some("google".to_string()), Some(cal.id))
            } else {
                (Some("local".to_string()), None)
            };

            queries::CreateSubjectEventPayload {
                subject,
                name,
                event_type,
                event_date,
                event_time: if event_time.is_empty() { None } else { Some(event_time) },
                end_date: if end_date.is_empty() { None } else { Some(end_date) },
                end_time: if end_time.is_empty() { None } else { Some(end_time) },
                is_all_day,
                calendar_type,
                google_calendar_id,
                notes: if notes.is_empty() { None } else { Some(notes) },
            }
        };

        let event = {
            let conn = db.lock().map_err(db_err)?;
            queries::subject_event_create(&conn, payload).map_err(db_err)?
        };

        if event.calendar_type == "google" {
            if let Some(ref cal_id) = event.google_calendar_id {
                if let Ok(token) = crate::google_calendar::auth::ensure_valid_token(&db).await {
                    let summary = format!("{}: {}", event.subject, event.name);
                    let res = crate::google_calendar::api::create_single_event(
                        &token,
                        cal_id,
                        &summary,
                        event.notes.as_deref(),
                        &event.event_date,
                        event.event_time.as_deref(),
                        event.end_date.as_deref(),
                        event.end_time.as_deref(),
                        event.is_all_day,
                        None,
                    ).await;

                    if let Ok(g_event_id) = res {
                        if let Ok(conn) = db.lock() {
                            let _ = queries::subject_event_update(&conn, event.id, queries::UpdateSubjectEventPayload {
                                google_event_id: Some(g_event_id),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }

        self.broadcast_events_changed().await;
        Ok(event.id)
    }

    /// Toggle completion state of an academic event by ID.
    async fn subject_event_toggle_completed(&self, id: i64) -> fdo::Result<bool> {
        let db = self.get_db()?;
        let new_status = {
            let conn = db.lock().map_err(db_err)?;
            let ev = queries::subject_event_get_by_id(&conn, id).map_err(db_err)?
                .ok_or_else(|| fdo::Error::Failed(format!("Event #{id} not found")))?;
            let new_status = !ev.is_completed;
            queries::subject_event_toggle_completed(&conn, id, new_status).map_err(db_err)?;
            new_status
        };
        self.broadcast_events_changed().await;
        Ok(new_status)
    }

    /// Delete an academic event by ID.
    async fn subject_event_delete(&self, id: i64) -> fdo::Result<()> {
        let db = self.get_db()?;
        let existing = {
            let conn = db.lock().map_err(db_err)?;
            let ev = queries::subject_event_get_by_id(&conn, id).map_err(db_err)?;
            queries::subject_event_delete(&conn, id).map_err(db_err)?;
            ev
        };

        if let Some(ev) = existing {
            if ev.calendar_type == "google" {
                if let (Some(ref cal_id), Some(ref g_event_id)) = (&ev.google_calendar_id, &ev.google_event_id) {
                    if let Ok(token) = crate::google_calendar::auth::ensure_valid_token(&db).await {
                        let _ = crate::google_calendar::api::delete_event(&token, cal_id, g_event_id).await;
                    }
                }
            }
        }

        self.broadcast_events_changed().await;
        Ok(())
    }

    /// Trigger background synchronization for academic events with Google Calendar.
    async fn subject_events_sync(&self) -> fdo::Result<()> {
        let db = self.get_db()?;
        if let Ok(token) = crate::google_calendar::auth::ensure_valid_token(&db).await {
            let cal = {
                let conn = db.lock().map_err(db_err)?;
                queries::get_google_events_calendar(&conn).map_err(db_err)?
            };
            if let Some(c) = cal {
                let _ = crate::google_calendar::sync::sync_subject_events(&db, &token, &c.id).await;
                self.broadcast_events_changed().await;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Timetable & Scheduled Planning Blocks
    // -----------------------------------------------------------------------

    /// Fetch all scheduled blocks across the entire week as JSON.
    async fn get_scheduled_blocks(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let blocks = queries::schedule_get_all(&conn).map_err(db_err)?;
        serde_json::to_string(&blocks).map_err(json_err)
    }

    /// Fetch today's scheduled blocks ordered by start time as JSON.
    async fn get_today_scheduled_blocks(&self) -> fdo::Result<String> {
        let today_dow = chrono::Local::now().weekday().num_days_from_monday() as i32;
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let all_blocks = queries::schedule_get_all(&conn).map_err(db_err)?;
        let today_blocks: Vec<_> = all_blocks.into_iter().filter(|b| b.day_of_week == today_dow).collect();
        serde_json::to_string(&today_blocks).map_err(json_err)
    }

    // -----------------------------------------------------------------------
    // Statistics & Analytics
    // -----------------------------------------------------------------------

    /// Fetch full detailed stats (today summary, current week, and streak) as JSON.
    async fn get_detailed_stats(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let today = queries::get_daily_stats(&conn).map_err(db_err)?;
        let week = queries::get_weekly_stats(&conn).map_err(db_err)?;
        let streak = queries::get_streak(&conn).map_err(db_err)?;
        let detailed = DetailedStats { today, week, streak };
        serde_json::to_string(&detailed).map_err(json_err)
    }

    /// Fetch concise metrics for today as a dictionary.
    async fn get_today_stats(&self) -> fdo::Result<HashMap<String, Value<'static>>> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let today = queries::get_daily_stats(&conn).map_err(db_err)?;
        let streak = queries::get_streak(&conn).map_err(db_err)?;
        let mut map = HashMap::new();
        map.insert("rounds".into(), Value::from(today.rounds));
        map.insert("focus_mins".into(), Value::from(today.focus_mins as u64));
        if let Some(rate) = today.completion_rate {
            map.insert("completion_rate".into(), Value::from(rate as f64));
        }
        map.insert("current_streak".into(), Value::from(streak.current as u64));
        map.insert("longest_streak".into(), Value::from(streak.longest as u64));
        Ok(map)
    }

    /// Fetch active streak information `(current_streak_days, longest_streak_days)`.
    async fn get_streak(&self) -> fdo::Result<(u32, u32)> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let streak = queries::get_streak(&conn).map_err(db_err)?;
        Ok((streak.current, streak.longest))
    }

    /// Fetch daily statistics for the current week as a JSON array.
    async fn get_weekly_stats(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let weekly = queries::get_weekly_stats(&conn).map_err(db_err)?;
        serde_json::to_string(&weekly).map_err(json_err)
    }

    // -----------------------------------------------------------------------
    // Subjects & Goals
    // -----------------------------------------------------------------------

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

    /// Fetch all tracked subjects along with total pomodoros and weekly goal as JSON.
    async fn get_subjects_with_stats(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let subjects = queries::subjects_get_all(&conn).map_err(db_err)?;
        serde_json::to_string(&subjects).map_err(json_err)
    }

    /// Fetch weekly goal and completed rounds for a specific subject as a dictionary.
    async fn get_subject_weekly_progress(&self, name: String) -> fdo::Result<HashMap<String, Value<'static>>> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let progress = queries::subject_get_weekly_progress(&conn, &name).map_err(db_err)?;
        let mut map = HashMap::new();
        map.insert("goal".into(), Value::from(progress.goal.unwrap_or(0)));
        map.insert("has_goal".into(), Value::from(progress.goal.is_some()));
        map.insert("completed".into(), Value::from(progress.completed));
        Ok(map)
    }

    /// Create a new study subject.
    async fn create_subject(&self, name: String) -> fdo::Result<i64> {
        let db = self.get_db()?;
        let id = {
            let conn = db.lock().map_err(db_err)?;
            queries::subject_create(&conn, &name).map_err(db_err)?
        };
        self.broadcast_subjects_changed().await;
        Ok(id)
    }

    /// Delete a study subject if it has no active (non-deleted) pomodoro rounds.
    async fn delete_subject(&self, name: String) -> fdo::Result<()> {
        let db = self.get_db()?;
        {
            let conn = db.lock().map_err(db_err)?;
            let count: u32 = conn.query_row(
                "SELECT COUNT(id) FROM rounds WHERE subject = ?1 AND round_type = 'work' AND deleted_at IS NULL",
                rusqlite::params![name],
                |r| r.get(0),
            ).map_err(db_err)?;

            if count > 0 {
                return Err(fdo::Error::Failed("Cannot delete subject with existing pomodoros".into()));
            }

            conn.execute("DELETE FROM subjects WHERE name = ?1", rusqlite::params![name]).map_err(db_err)?;
        }
        self.broadcast_subjects_changed().await;
        Ok(())
    }

    /// Set or clear the weekly round goal for a subject (`goal = 0` clears the goal).
    async fn set_subject_weekly_goal(&self, name: String, goal: u32) -> fdo::Result<()> {
        let db = self.get_db()?;
        {
            let conn = db.lock().map_err(db_err)?;
            let target_goal = if goal == 0 { None } else { Some(goal) };
            queries::subject_set_weekly_goal(&conn, &name, target_goal).map_err(db_err)?;
        }
        self.broadcast_subjects_changed().await;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Settings & Themes
    // -----------------------------------------------------------------------

    /// Read any setting value from SQLite by key.
    async fn get_setting(&self, key: String) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let val = settings::get_setting(&conn, &key);
        Ok(val.unwrap_or_default())
    }

    /// Save a setting value and broadcast the update to the app.
    async fn set_setting(&self, key: String, value: String) -> fdo::Result<()> {
        let db = self.get_db()?;
        {
            let conn = db.lock().map_err(db_err)?;
            settings::save_setting(&conn, &key, &value).map_err(db_err)?;
            if let Ok(new_settings) = settings::load(&conn) {
                let _ = self.app.emit("settings:changed", &new_settings);
            }
        }
        self.broadcast_setting_changed(&key, &value).await;
        Ok(())
    }

    /// List all available theme names (bundled + custom).
    async fn get_themes(&self) -> fdo::Result<Vec<String>> {
        let data_dir = self.app.path().app_data_dir().map_err(db_err)?;
        let themes = themes::list_all(&data_dir);
        Ok(themes.into_iter().map(|t| t.name).collect())
    }

    /// Get the active theme name.
    async fn get_current_theme(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let s = settings::load(&conn).map_err(db_err)?;
        let theme_name = match s.theme_mode.as_str() {
            "dark" => s.theme_dark,
            _ => s.theme_light,
        };
        Ok(theme_name)
    }

    /// Get current theme colors as a JSON map (e.g. `--color-background`, `--color-accent`).
    async fn get_current_theme_colors(&self) -> fdo::Result<String> {
        let db = self.get_db()?;
        let theme_name = {
            let conn = db.lock().map_err(db_err)?;
            let s = settings::load(&conn).map_err(db_err)?;
            match s.theme_mode.as_str() {
                "dark" => s.theme_dark,
                _ => s.theme_light,
            }
        };
        let data_dir = self.app.path().app_data_dir().map_err(db_err)?;
        let theme = themes::find(&data_dir, &theme_name)
            .or_else(|| themes::find(&data_dir, "Pomotroid"))
            .ok_or_else(|| fdo::Error::Failed("Theme not found".into()))?;
        serde_json::to_string(&theme.colors).map_err(json_err)
    }

    /// Change the active theme.
    async fn set_theme(&self, theme_name: String) -> fdo::Result<()> {
        log::info!("[dbus] SetTheme called: {theme_name}");
        let data_dir = self.app.path().app_data_dir().map_err(db_err)?;
        if themes::find(&data_dir, &theme_name).is_none() {
            return Err(fdo::Error::Failed(format!("Theme '{theme_name}' not found")));
        }
        let db = self.get_db()?;
        let s = {
            let conn = db.lock().map_err(db_err)?;
            let mut s = settings::load(&conn).map_err(db_err)?;
            if s.theme_mode == "dark" {
                settings::save_setting(&conn, "theme_dark", &theme_name).map_err(db_err)?;
                s.theme_dark = theme_name.clone();
            } else {
                settings::save_setting(&conn, "theme_light", &theme_name).map_err(db_err)?;
                s.theme_light = theme_name.clone();
            }
            s
        };
        let _ = self.app.emit("settings:changed", &s);
        self.broadcast_theme_changed(&theme_name).await;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Google Calendar
    // -----------------------------------------------------------------------

    /// Trigger immediate bidirectional synchronization with Google Calendar.
    async fn google_calendar_sync_now(&self) -> fdo::Result<()> {
        log::info!("[dbus] GoogleCalendarSyncNow called");
        let db = self.get_db()?;
        let token = match crate::google_calendar::auth::ensure_valid_token(&db).await {
            Ok(tok) => tok,
            Err(e) => return Err(fdo::Error::Failed(format!("Google Calendar auth error: {e}"))),
        };
        let (synced_cal, events_cal) = {
            let conn = db.lock().map_err(db_err)?;
            (
                queries::get_google_synced_calendar(&conn).map_err(db_err)?,
                queries::get_google_events_calendar(&conn).map_err(db_err)?,
            )
        };
        if let Some(cal) = events_cal {
            let _ = crate::google_calendar::sync::sync_subject_events(&db, &token, &cal.id).await;
            self.broadcast_events_changed().await;
        }
        if let Some(cal) = synced_cal {
            let monday_ymd = crate::google_calendar::sync::current_monday_ymd();
            let _ = crate::google_calendar::sync::sync_calendar_two_way(&db, &token, &cal.id, &monday_ymd).await;
            self.broadcast_schedule_changed().await;
        }
        Ok(())
    }

    /// Fetch Google Calendar connection status and configured calendars.
    async fn google_calendar_get_status(&self) -> fdo::Result<HashMap<String, Value<'static>>> {
        let db = self.get_db()?;
        let conn = db.lock().map_err(db_err)?;
        let auth = queries::get_google_auth(&conn).map_err(db_err)?;
        let synced_cal = queries::get_google_synced_calendar(&conn).map_err(db_err)?;
        let events_cal = queries::get_google_events_calendar(&conn).map_err(db_err)?;
        let mut map = HashMap::new();
        map.insert("connected".into(), Value::from(auth.is_some()));
        map.insert("email".into(), Value::from(auth.and_then(|a| a.email).unwrap_or_default()));
        map.insert("has_synced_calendar".into(), Value::from(synced_cal.is_some()));
        map.insert("synced_calendar_summary".into(), Value::from(synced_cal.map(|c| c.summary).unwrap_or_default()));
        map.insert("has_events_calendar".into(), Value::from(events_cal.is_some()));
        map.insert("events_calendar_summary".into(), Value::from(events_cal.map(|c| c.summary).unwrap_or_default()));
        Ok(map)
    }

    // -----------------------------------------------------------------------
    // Windows
    // -----------------------------------------------------------------------

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

    /// Open or focus the Pomotroid settings window, optionally navigating to a section.
    async fn open_settings_window(&self, section: String) {
        log::info!("[dbus] OpenSettingsWindow called (section='{section}')");
        if let Some(existing) = self.app.get_webview_window("settings") {
            let _ = existing.show();
            let _ = existing.set_focus();
            if !section.is_empty() {
                let _ = existing.emit("settings:navigate", &section);
            }
            return;
        }

        let is_mac = cfg!(target_os = "macos");
        let url = if section.is_empty() {
            "/settings".to_string()
        } else {
            format!("/settings?section={}", urlencoding::encode(&section))
        };
        let win = tauri::WebviewWindowBuilder::new(&self.app, "settings", tauri::WebviewUrl::App(url.into()))
            .title("Pomotroid — Settings")
            .inner_size(720.0, 520.0)
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::DetailedStats;
    use crate::db::queries::{DailyStats, DayStat, StreakInfo};

    #[test]
    fn test_dbus_state_initialization() {
        let state = DbusState::new(4);
        assert_eq!(state.goal_rounds.load(Ordering::Relaxed), 4);
        let tags = state.pending_tags.lock().unwrap();
        assert_eq!(tags.subject, "");
        assert_eq!(tags.subject_topic, "");
        assert_eq!(tags.study_type, "");
        assert_eq!(tags.notes, "");
    }

    #[test]
    fn test_detailed_stats_serialization() {
        let stats = DetailedStats {
            today: DailyStats {
                rounds: 3.5,
                focus_mins: 87,
                completion_rate: Some(0.95),
                by_hour: vec![0.0; 24],
            },
            week: vec![
                DayStat {
                    date: "2026-09-22".to_string(),
                    rounds: 4.0,
                }
            ],
            streak: StreakInfo {
                current: 5,
                longest: 12,
            },
        };

        let json_str = serde_json::to_string(&stats).expect("should serialize DetailedStats");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should parse json");

        assert_eq!(parsed["today"]["rounds"], 3.5);
        assert_eq!(parsed["today"]["focus_mins"], 87);
        assert_eq!(parsed["streak"]["current"], 5);
        assert_eq!(parsed["streak"]["longest"], 12);
        assert_eq!(parsed["week"][0]["rounds"], 4.0);
    }
}

