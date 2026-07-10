use rusqlite::{params, Connection, Result, OptionalExtension};
use serde::{Serialize, Deserialize};


pub fn insert_study_session(
    conn: &Connection,
    goal_rounds: u32,
    subject: Option<String>,
    subject_topic: Option<String>,
    study_type: Option<String>,
) -> Result<i64> {
    let started_at = unix_now();
    let uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO study_sessions (uuid, started_at, created_at, goal_rounds, subject, subject_topic, study_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![uuid, started_at, started_at, goal_rounds, subject, subject_topic, study_type],
    )?;
    let id = conn.last_insert_rowid();
    log::debug!("[db] study_session started: id={id} goal={goal_rounds}");
    Ok(id)
}

pub fn complete_study_session(
    conn: &Connection,
    session_id: i64,
) -> Result<()> {
    let now = unix_now();
    conn.execute(
        "UPDATE study_sessions SET ended_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![now, now, session_id],
    )?;
    log::debug!("[db] study_session ended: id={session_id}");
    Ok(())
}

pub fn add_pause_time(
    conn: &Connection,
    round_id: i64,
    session_id: Option<i64>,
    pause_duration: u32,
) -> Result<()> {
    conn.execute(
        "UPDATE rounds SET pause_secs = pause_secs + ?1 WHERE id = ?2",
        params![pause_duration, round_id],
    )?;
    if let Some(sid) = session_id {
        conn.execute(
            "UPDATE study_sessions SET total_pause_secs = total_pause_secs + ?1 WHERE id = ?2",
            params![pause_duration, sid],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Session CRUD (DATA-03)
// ---------------------------------------------------------------------------

/// Inserts a new session row when a round begins.
/// Returns the row ID so it can be passed to `complete_session` later.
pub fn insert_round(
    conn: &Connection,
    study_session_id: Option<i64>,
    round_type: &str,
    duration_secs: u32,
) -> Result<i64> {
    let started_at = unix_now();
    let uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO rounds (uuid, started_at, updated_at, round_type, duration_secs, completed, study_session_id)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
        params![uuid, started_at, started_at, round_type, duration_secs, study_session_id],
    )?;
    let id = conn.last_insert_rowid();
    log::debug!("[db] session started: id={id} type={round_type} duration={duration_secs}s");
    Ok(id)
}

/// Updates a session when the round ends (by completion or skip).
pub fn complete_round(
    conn: &Connection,
    session_id: i64,
    completed: bool,
) -> Result<()> {
    let now = unix_now();
    conn.execute(
        "UPDATE rounds SET ended_at = ?1, completed = ?2, updated_at = ?3 WHERE id = ?4",
        params![now, completed as i64, now, session_id],
    )?;
    log::debug!("[db] session ended: id={session_id} completed={completed}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Extended Session CRUD
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: i64,
    pub uuid: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub round_type: String,
    pub duration_secs: u32,
    pub completed: bool,
    pub subject: Option<String>,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub notes: Option<String>,
    pub updated_at: Option<i64>,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionPayload {
    pub subject: Option<String>,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStudySessionPayload {
    pub subject: Option<String>,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub notes: Option<String>,
    pub goal_rounds: Option<u32>,
}


#[derive(Debug, Deserialize)]
pub struct SessionFilter {
    pub subject: Option<String>,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub show_breaks: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StudySessionRow {
    pub id: i64,
    pub uuid: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub goal_rounds: u32,
    pub total_pause_secs: u32,
    pub subject: Option<String>,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub deleted_at: Option<i64>,
    pub rounds: Vec<SessionRow>,
}

#[derive(Debug, Serialize)]
pub struct SessionHistoryPage {
    pub sessions: Vec<StudySessionRow>,
    pub total: u32,
    pub total_work_rounds: f64,
    pub total_focus_secs: u64,
    pub longest_streak: u32,
}

pub fn get_history(
    conn: &Connection,
    limit: u32,
    offset: u32,
    filter: &SessionFilter,
) -> Result<SessionHistoryPage> {
    let mut query = String::from("SELECT id, uuid, started_at, ended_at, goal_rounds, total_pause_secs, subject, subject_topic, study_type, notes, created_at, updated_at, deleted_at FROM study_sessions WHERE deleted_at IS NULL");
    let mut count_query = String::from("SELECT COUNT(*) FROM study_sessions WHERE deleted_at IS NULL");
    
    let mut params = Vec::<rusqlite::types::Value>::new();

    if let Some(subject) = &filter.subject {
        let sql = format!(" AND subject = ?{}", params.len() + 1);
        query.push_str(&sql);
        count_query.push_str(&sql);
        params.push(subject.clone().into());
    }
    if let Some(topic) = &filter.subject_topic {
        let sql = format!(" AND subject_topic = ?{}", params.len() + 1);
        query.push_str(&sql);
        count_query.push_str(&sql);
        params.push(topic.clone().into());
    }
    if let Some(stype) = &filter.study_type {
        let sql = format!(" AND study_type = ?{}", params.len() + 1);
        query.push_str(&sql);
        count_query.push_str(&sql);
        params.push(stype.clone().into());
    }
    if let Some(d_from) = filter.date_from {
        let sql = format!(" AND started_at >= ?{}", params.len() + 1);
        query.push_str(&sql);
        count_query.push_str(&sql);
        params.push(d_from.into());
    }
    if let Some(d_to) = filter.date_to {
        let sql = format!(" AND started_at <= ?{}", params.len() + 1);
        query.push_str(&sql);
        count_query.push_str(&sql);
        params.push(d_to.into());
    }
    // Note: We ignore filter.show_breaks for the study_sessions query, since a study session usually has both work and breaks.
    
    let total: u32 = conn.query_row(&count_query, rusqlite::params_from_iter(params.iter()), |row| row.get(0))?;

    let sql = format!(" ORDER BY started_at DESC, id DESC LIMIT ?{} OFFSET ?{}", params.len() + 1, params.len() + 2);
    query.push_str(&sql);
    params.push(limit.into());
    params.push(offset.into());

    let mut stmt = conn.prepare(&query)?;
    let iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(StudySessionRow {
            id: row.get(0)?,
            uuid: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            goal_rounds: row.get(4)?,
            total_pause_secs: row.get(5)?,
            subject: row.get(6)?,
            subject_topic: row.get(7)?,
            study_type: row.get(8)?,
            notes: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
            deleted_at: row.get(12)?,
            rounds: Vec::new(),
        })
    })?;

    let mut sessions = Vec::new();
    for row in iter {
        let mut session = row?;
        
        let mut rounds_query = String::from("SELECT id, uuid, started_at, ended_at, round_type, duration_secs, completed, subject, subject_topic, study_type, notes, updated_at, deleted_at FROM rounds WHERE study_session_id = ?1 AND deleted_at IS NULL");
        if let Some(false) = filter.show_breaks {
             rounds_query.push_str(" AND round_type = 'work'");
        }
        rounds_query.push_str(" ORDER BY started_at ASC");
        
        let mut r_stmt = conn.prepare(&rounds_query)?;
        let r_iter = r_stmt.query_map([session.id], |row| {
             Ok(SessionRow {
                 id: row.get(0)?,
                 uuid: row.get(1)?,
                 started_at: row.get(2)?,
                 ended_at: row.get(3)?,
                 round_type: row.get(4)?,
                 duration_secs: row.get(5)?,
                 completed: row.get::<_, i64>(6)? != 0,
                 subject: row.get(7)?,
                 subject_topic: row.get(8)?,
                 study_type: row.get(9)?,
                 notes: row.get(10)?,
                 updated_at: row.get(11)?,
                 deleted_at: row.get(12)?,
             })
        })?;
        
        for r in r_iter {
            session.rounds.push(r?);
        }
        sessions.push(session);
    }

    // Compute stats for all filtered WORK sessions
    let mut stats_query = String::from("SELECT started_at, duration_secs FROM rounds WHERE deleted_at IS NULL AND round_type = 'work' AND completed = 1");
    let mut stats_params = Vec::<rusqlite::types::Value>::new();

    if let Some(subject) = &filter.subject {
        let sql = format!(" AND subject = ?{}", stats_params.len() + 1);
        stats_query.push_str(&sql);
        stats_params.push(subject.clone().into());
    }
    if let Some(topic) = &filter.subject_topic {
        let sql = format!(" AND subject_topic = ?{}", stats_params.len() + 1);
        stats_query.push_str(&sql);
        stats_params.push(topic.clone().into());
    }
    if let Some(stype) = &filter.study_type {
        let sql = format!(" AND study_type = ?{}", stats_params.len() + 1);
        stats_query.push_str(&sql);
        stats_params.push(stype.clone().into());
    }
    if let Some(d_from) = filter.date_from {
        let sql = format!(" AND started_at >= ?{}", stats_params.len() + 1);
        stats_query.push_str(&sql);
        stats_params.push(d_from.into());
    }
    if let Some(d_to) = filter.date_to {
        let sql = format!(" AND started_at <= ?{}", stats_params.len() + 1);
        stats_query.push_str(&sql);
        stats_params.push(d_to.into());
    }
    
    stats_query.push_str(" ORDER BY started_at ASC");

    let time_work_secs: u32 = conn.query_row("SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'", [], |r| r.get(0)).unwrap_or(1500);

    let mut total_work_rounds = 0f64;
    let mut total_focus_secs = 0u64;
    let mut days_set = std::collections::BTreeSet::new();

    let mut stats_stmt = conn.prepare(&stats_query)?;
    let stats_iter = stats_stmt.query_map(rusqlite::params_from_iter(stats_params.iter()), |row| {
        let started_at: i64 = row.get(0)?;
        let duration: u32 = row.get(1)?;
        Ok((started_at, duration))
    })?;

    for row in stats_iter {
        if let Ok((started_at, duration)) = row {
            total_focus_secs += duration as u64;
            let rounds = duration as f64 / time_work_secs as f64;
            total_work_rounds += rounds;
            if let Some(dt) = chrono::DateTime::from_timestamp(started_at, 0) {
                let local = dt.with_timezone(&chrono::Local);
                days_set.insert(local.format("%Y-%m-%d").to_string());
            }
        }
    }

    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let days: Vec<String> = days_set.into_iter().collect();
    let streak_info = compute_streak(&days, &today_str);

    Ok(SessionHistoryPage { 
        sessions, 
        total,
        total_work_rounds,
        total_focus_secs,
        longest_streak: streak_info.longest
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateManualSessionPayload {
    pub started_at: i64,
    pub duration_secs: u32,
    pub subject: Option<String>,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub notes: Option<String>,
}

pub fn get_session(conn: &Connection, id: i64) -> Result<Option<SessionRow>> {
    conn.query_row(
        "SELECT id, uuid, started_at, ended_at, round_type, duration_secs, completed,
                subject, subject_topic, study_type, notes, updated_at, deleted_at
         FROM rounds WHERE id = ?1",
        [id],
        |r| {
            Ok(SessionRow {
                id: r.get(0)?,
                uuid: r.get(1)?,
                started_at: r.get(2)?,
                ended_at: r.get(3)?,
                round_type: r.get(4)?,
                duration_secs: r.get(5)?,
                completed: r.get::<_, i64>(6)? != 0,
                subject: r.get(7)?,
                subject_topic: r.get(8)?,
                study_type: r.get(9)?,
                notes: r.get(10)?,
                updated_at: r.get(11)?,
                deleted_at: r.get(12)?,
            })
        },
    ).optional()
}

pub fn get_study_session(conn: &Connection, id: i64) -> Result<Option<StudySessionRow>> {
    let row = conn.query_row(
        "SELECT id, uuid, started_at, ended_at, goal_rounds, total_pause_secs, subject, subject_topic, study_type, notes, created_at, updated_at, deleted_at FROM study_sessions WHERE id = ?1",
        [id],
        |r| {
            Ok(StudySessionRow {
                id: r.get(0)?,
                uuid: r.get(1)?,
                started_at: r.get(2)?,
                ended_at: r.get(3)?,
                goal_rounds: r.get(4)?,
                total_pause_secs: r.get(5)?,
                subject: r.get(6)?,
                subject_topic: r.get(7)?,
                study_type: r.get(8)?,
                notes: r.get(9)?,
                created_at: r.get(10)?,
                updated_at: r.get(11)?,
                deleted_at: r.get(12)?,
                rounds: Vec::new(),
            })
        },
    ).optional()?;

    let mut session = match row {
        Some(s) => s,
        None => return Ok(None),
    };

    let mut r_stmt = conn.prepare("SELECT id, uuid, started_at, ended_at, round_type, duration_secs, completed, subject, subject_topic, study_type, notes, updated_at, deleted_at FROM rounds WHERE study_session_id = ?1 AND deleted_at IS NULL ORDER BY started_at ASC")?;
    let r_iter = r_stmt.query_map([session.id], |r| {
        Ok(SessionRow {
            id: r.get(0)?,
            uuid: r.get(1)?,
            started_at: r.get(2)?,
            ended_at: r.get(3)?,
            round_type: r.get(4)?,
            duration_secs: r.get(5)?,
            completed: r.get::<_, i64>(6)? != 0,
            subject: r.get(7)?,
            subject_topic: r.get(8)?,
            study_type: r.get(9)?,
            notes: r.get(10)?,
            updated_at: r.get(11)?,
            deleted_at: r.get(12)?,
        })
    })?;
    for r in r_iter {
        session.rounds.push(r?);
    }
    Ok(Some(session))
}

pub fn delete_session(conn: &Connection, id: i64) -> Result<()> {
    let now = unix_now();
    conn.execute(
        "UPDATE rounds SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![now, now, id],
    )?;
    Ok(())
}

pub fn update_session(conn: &Connection, id: i64, payload: UpdateSessionPayload) -> Result<()> {
    if let Some(subject_name) = &payload.subject {
        if !subject_name.trim().is_empty() {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO subjects (name, created_at) VALUES (?1, ?2)",
                params![subject_name.trim(), unix_now()],
            );
        }
    }
    conn.execute(
        "UPDATE rounds SET
            subject = ?1,
            subject_topic = ?2,
            study_type = ?3,
            notes = ?4,
            updated_at = ?5
         WHERE id = ?6",
        params![
            payload.subject,
            payload.subject_topic,
            payload.study_type,
            payload.notes,
            unix_now(),
            id
        ],
    )?;
    Ok(())
}

pub fn update_study_session(conn: &Connection, id: i64, payload: UpdateStudySessionPayload) -> Result<()> {
    if let Some(subject_name) = &payload.subject {
        if !subject_name.trim().is_empty() {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO subjects (name, created_at) VALUES (?1, ?2)",
                params![subject_name.trim(), unix_now()],
            );
        }
    }
    
    // Update the study_sessions row
    conn.execute(
        "UPDATE study_sessions SET
            subject = ?1,
            subject_topic = ?2,
            study_type = ?3,
            notes = ?4,
            goal_rounds = ?5,
            updated_at = ?6
         WHERE id = ?7",
        params![
            payload.subject,
            payload.subject_topic,
            payload.study_type,
            payload.notes,
            payload.goal_rounds,
            unix_now(),
            id
        ],
    )?;

    // Cascade ONLY subject to rounds that belong to this study session (as per user request)
    conn.execute(
        "UPDATE rounds SET
            subject = ?1,
            updated_at = ?2
         WHERE study_session_id = ?3",
        params![
            payload.subject,
            unix_now(),
            id
        ],
    )?;
    Ok(())
}

pub fn add_extra_time_to_round(conn: &Connection, id: i64, extra_secs: i64) -> Result<()> {
    conn.execute(
        "UPDATE rounds SET duration_secs = duration_secs + ?1, overtime_secs = overtime_secs + ?1, updated_at = ?2 WHERE id = ?3",
        params![extra_secs, unix_now(), id],
    )?;
    Ok(())
}

pub fn insert_manual_session(conn: &Connection, payload: CreateManualSessionPayload) -> Result<i64> {
    if let Some(subject_name) = &payload.subject {
        if !subject_name.trim().is_empty() {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO subjects (name, created_at) VALUES (?1, ?2)",
                params![subject_name.trim(), unix_now()],
            );
        }
    }

    let uuid = uuid::Uuid::new_v4().to_string();
    let ended_at = payload.started_at + (payload.duration_secs as i64);
    
    conn.execute(
        "INSERT INTO rounds (
            uuid, started_at, ended_at, round_type, duration_secs, completed,
            subject, subject_topic, study_type, notes, updated_at
        ) VALUES (
            ?1, ?2, ?3, 'work', ?4, 1,
            ?5, ?6, ?7, ?8, ?9
        )",
        params![
            uuid,
            payload.started_at,
            ended_at,
            payload.duration_secs,
            payload.subject,
            payload.subject_topic,
            payload.study_type,
            payload.notes,
            unix_now()
        ],
    )?;
    
    let id = conn.last_insert_rowid();
    log::debug!("[db] manual session inserted: id={id} duration={}", payload.duration_secs);
    Ok(id)
}

pub fn get_distinct_subjects(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM subjects ORDER BY name COLLATE NOCASE ASC"
    )?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    let mut subjects = Vec::new();
    for row in rows {
        subjects.push(row?);
    }
    Ok(subjects)
}

#[derive(Debug, Serialize)]
pub struct SubjectStats {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub pomodoro_count: u32,
    pub weekly_goal: Option<u32>,
}

pub fn subjects_get_all(conn: &Connection) -> Result<Vec<SubjectStats>> {
    let mut stmt = conn.prepare(
        "SELECT 
            sb.id, 
            sb.name, 
            sb.color,
            COUNT(se.id) as pomodoro_count,
            sb.weekly_goal
         FROM subjects sb
         LEFT JOIN rounds se ON se.subject = sb.name AND se.round_type = 'work' AND se.completed = 1 AND se.deleted_at IS NULL
         GROUP BY sb.id
         ORDER BY sb.name COLLATE NOCASE ASC"
    )?;
    
    let rows = stmt.query_map([], |row| {
        Ok(SubjectStats {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            pomodoro_count: row.get(3)?,
            weekly_goal: row.get(4)?,
        })
    })?;

    let mut subjects = Vec::new();
    for row in rows {
        subjects.push(row?);
    }
    Ok(subjects)
}

pub fn subject_create(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO subjects (name, created_at) VALUES (?1, ?2)",
        params![name.trim(), unix_now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn subject_set_weekly_goal(conn: &Connection, name: &str, goal: Option<u32>) -> Result<()> {
    conn.execute(
        "UPDATE subjects SET weekly_goal = ?1 WHERE name = ?2",
        params![goal, name.trim()],
    )?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct SubjectWeeklyProgress {
    pub goal: Option<u32>,
    pub completed: u32,
}

pub fn subject_get_weekly_progress(conn: &Connection, name: &str) -> Result<SubjectWeeklyProgress> {
    let goal: Option<u32> = conn.query_row(
        "SELECT weekly_goal FROM subjects WHERE name = ?1",
        params![name.trim()],
        |r| r.get(0)
    ).optional()?.flatten();

    let completed: u32 = conn.query_row(
        "SELECT COUNT(id) FROM rounds 
         WHERE subject = ?1 
         AND round_type = 'work' 
         AND completed = 1 
         AND deleted_at IS NULL
         AND date(started_at, 'unixepoch', 'localtime') >= date('now', 'localtime', '-3 days', 'weekday 4', '-3 days')",
        params![name.trim()],
        |r| r.get(0)
    ).unwrap_or(0);

    Ok(SubjectWeeklyProgress { goal, completed })
}

pub fn get_distinct_topics(conn: &Connection, subject: Option<&str>) -> Result<Vec<String>> {
    let mut topics = Vec::new();
    if let Some(s) = subject {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT subject_topic FROM rounds WHERE subject = ?1 AND subject_topic IS NOT NULL AND subject_topic != '' AND deleted_at IS NULL ORDER BY subject_topic COLLATE NOCASE ASC"
        )?;
        let rows = stmt.query_map([s], |r| r.get(0))?;
        for row in rows {
            topics.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT subject_topic FROM rounds WHERE subject_topic IS NOT NULL AND subject_topic != '' AND deleted_at IS NULL ORDER BY subject_topic COLLATE NOCASE ASC"
        )?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        for row in rows {
            topics.push(row?);
        }
    }
    Ok(topics)
}

pub fn get_distinct_study_types(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT study_type FROM rounds WHERE study_type IS NOT NULL AND study_type != '' AND deleted_at IS NULL ORDER BY study_type COLLATE NOCASE ASC"
    )?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    let mut types = Vec::new();
    for row in rows {
        types.push(row?);
    }
    Ok(types)
}

// ---------------------------------------------------------------------------
// Stats queries
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total_work_sessions: f64,
    pub completed_work_sessions: f64,
    /// Sum of duration_secs for all *completed* work sessions.
    pub total_work_secs: i64,
}

pub fn get_all_time_stats(conn: &Connection) -> Result<SessionStats> {
    let total_work_sessions: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) FROM rounds WHERE round_type = 'work' AND deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;

    let completed_work_sessions: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) FROM rounds WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;

    let total_work_secs: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0)
         FROM rounds WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;

    Ok(SessionStats {
        total_work_sessions,
        completed_work_sessions,
        total_work_secs,
    })
}

// ---------------------------------------------------------------------------
// Detailed stats queries (DATA-04)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DailyStats {
    pub rounds: f64,
    pub focus_mins: u32,
    /// None when no work sessions were started today (avoids 0/0).
    pub completion_rate: Option<f32>,
    /// Completed work rounds per hour of the day (index 0 = midnight).
    pub by_hour: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct DayStat {
    /// Local calendar date in "YYYY-MM-DD" format.
    pub date: String,
    pub rounds: f64,
}

#[derive(Debug, Serialize)]
pub struct HeatmapEntry {
    /// Local calendar date in "YYYY-MM-DD" format.
    pub date: String,
    pub count: f64,
    pub focus_secs: u32,
}

#[derive(Debug, Serialize)]
pub struct StreakInfo {
    pub current: u32,
    pub longest: u32,
}

/// Completed work rounds and focus time for today (local calendar date).
pub fn get_daily_stats(conn: &Connection) -> Result<DailyStats> {
    let today: String = conn.query_row(
        "SELECT date('now', 'localtime')",
        [],
        |r| r.get(0),
    )?;

    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) FROM rounds
         WHERE round_type = 'work' AND deleted_at IS NULL
         AND date(started_at, 'unixepoch', 'localtime') = ?1",
        [&today],
        |r| r.get(0),
    )?;

    let completed: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) FROM rounds
         WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL
         AND date(started_at, 'unixepoch', 'localtime') = ?1",
        [&today],
        |r| r.get(0),
    )?;

    let focus_secs: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM rounds
         WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL
         AND date(started_at, 'unixepoch', 'localtime') = ?1",
        [&today],
        |r| r.get(0),
    )?;

    let mut by_hour = vec![0f64; 24];
    let mut stmt = conn.prepare(
        "SELECT CAST(strftime('%H', datetime(started_at, 'unixepoch', 'localtime')) AS INTEGER) as h,
                COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) as cnt
         FROM rounds
         WHERE round_type = 'work' AND completed = 1
         AND date(started_at, 'unixepoch', 'localtime') = ?1
         GROUP BY h",
    )?;
    let rows = stmt.query_map([&today], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))?;
    for row in rows.flatten() {
        let (h, cnt) = row;
        if (0..24).contains(&h) {
            by_hour[h as usize] = cnt;
        }
    }

    Ok(DailyStats {
        rounds: completed,
        focus_mins: ((focus_secs + 30) / 60) as u32,
        completion_rate: if total > 0.0 { Some((completed / total) as f32) } else { None },
        by_hour,
    })
}

/// Completed work rounds per local calendar day for the last 7 days.
pub fn get_weekly_stats(conn: &Connection) -> Result<Vec<DayStat>> {
    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') as day,
                COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) as rounds
         FROM rounds
         WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL
         AND date(started_at, 'unixepoch', 'localtime') >= date('now', 'localtime', '-6 days')
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt.query_map([], |r| Ok(DayStat { date: r.get(0)?, rounds: r.get(1)? }))?
        .collect();
    rows
}

/// Completed work rounds per local calendar day, all time (no date limit).
/// The frontend slices this into per-year views for navigation.
pub fn get_heatmap_data(conn: &Connection) -> Result<Vec<HeatmapEntry>> {
    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') as day,
                COALESCE(SUM(CAST(duration_secs AS REAL) / COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'time_work_secs'), 1500)), 0) as cnt,
                COALESCE(SUM(duration_secs), 0) as focus_secs
         FROM rounds
         WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt.query_map([], |r| Ok(HeatmapEntry { date: r.get(0)?, count: r.get(1)?, focus_secs: r.get(2)? }))?
        .collect();
    rows
}

/// Current and longest work-session streaks (consecutive local calendar days).
/// A streak stays active until midnight: if yesterday had sessions but today does not,
/// the streak is still counted as current.
pub fn get_streak(conn: &Connection) -> Result<StreakInfo> {
    let today: String = conn.query_row(
        "SELECT date('now', 'localtime')",
        [],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') as day
         FROM rounds
         WHERE round_type = 'work' AND completed = 1 AND deleted_at IS NULL
         GROUP BY day
         ORDER BY day",
    )?;
    let days: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .flatten()
        .collect();

    Ok(compute_streak(&days, &today))
}

// ---------------------------------------------------------------------------
// Streak helpers
// ---------------------------------------------------------------------------

/// Convert a "YYYY-MM-DD" string to a day number for arithmetic comparison.
/// Uses the proleptic Gregorian calendar; absolute value is arbitrary — only
/// differences between dates matter.
fn date_to_day_num(s: &str) -> Option<i32> {
    let mut parts = s.splitn(3, '-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: i32 = parts.next()?.parse().ok()?;
    let d: i32 = parts.next()?.parse().ok()?;
    let y = if m <= 2 { y - 1 } else { y };
    let m = if m <= 2 { m + 12 } else { m };
    Some(y * 365 + y / 4 - y / 100 + y / 400 + (153 * m - 457) / 5 + d)
}

pub fn compute_streak(days: &[String], today: &str) -> StreakInfo {
    let nums: Vec<i32> = days.iter().filter_map(|s| date_to_day_num(s)).collect();
    if nums.is_empty() {
        return StreakInfo { current: 0, longest: 0 };
    }

    let today_n = match date_to_day_num(today) {
        Some(n) => n,
        None => return StreakInfo { current: 0, longest: 0 },
    };

    // Current streak — alive if most recent session day is today or yesterday.
    let last = *nums.last().unwrap();
    let current = if last == today_n || last == today_n - 1 {
        let mut count = 0u32;
        let mut expected = last;
        for &n in nums.iter().rev() {
            if n == expected {
                count += 1;
                expected -= 1;
            } else {
                break;
            }
        }
        count
    } else {
        0
    };

    // Longest streak.
    let mut longest = 1u32;
    let mut run = 1u32;
    for i in 1..nums.len() {
        if nums[i] == nums[i - 1] + 1 {
            run += 1;
            if run > longest { longest = run; }
        } else {
            run = 1;
        }
    }

    StreakInfo { current, longest }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn insert_and_complete_session() {
        let conn = setup();
        let id = insert_session(&conn, "work", 1500).unwrap();
        assert!(id > 0);

        complete_session(&conn, id, true).unwrap();

        let completed: i64 = conn
            .query_row(
                "SELECT completed FROM rounds WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(completed, 1);
    }

    #[test]
    fn stats_empty_db() {
        let conn = setup();
        let stats = get_all_time_stats(&conn).unwrap();
        assert_eq!(stats.total_work_sessions, 0.0);
        assert_eq!(stats.completed_work_sessions, 0.0);
        assert_eq!(stats.total_work_secs, 0);
    }

    #[test]
    fn compute_streak_empty() {
        let info = compute_streak(&[], "2024-03-15");
        assert_eq!(info.current, 0);
        assert_eq!(info.longest, 0);
    }

    #[test]
    fn compute_streak_active_today() {
        let days = vec!["2024-03-13".to_string(), "2024-03-14".to_string(), "2024-03-15".to_string()];
        let info = compute_streak(&days, "2024-03-15");
        assert_eq!(info.current, 3);
        assert_eq!(info.longest, 3);
    }

    #[test]
    fn compute_streak_active_until_midnight() {
        // Yesterday had sessions, today does not — streak still live.
        let days = vec!["2024-03-13".to_string(), "2024-03-14".to_string()];
        let info = compute_streak(&days, "2024-03-15");
        assert_eq!(info.current, 2);
    }

    #[test]
    fn compute_streak_broken() {
        // Last session was 2 days ago — streak is broken.
        let days = vec!["2024-03-12".to_string(), "2024-03-13".to_string()];
        let info = compute_streak(&days, "2024-03-15");
        assert_eq!(info.current, 0);
    }

    #[test]
    fn compute_streak_longest_across_break() {
        let days = vec![
            "2024-03-01".to_string(), "2024-03-02".to_string(), "2024-03-03".to_string(),
            "2024-03-10".to_string(), "2024-03-11".to_string(),
        ];
        let info = compute_streak(&days, "2024-03-11");
        assert_eq!(info.current, 2);
        assert_eq!(info.longest, 3);
    }

    #[test]
    fn get_daily_stats_empty() {
        let conn = setup();
        let stats = get_daily_stats(&conn).unwrap();
        assert_eq!(stats.rounds, 0.0);
        assert_eq!(stats.focus_mins, 0);
        assert!(stats.completion_rate.is_none());
        assert_eq!(stats.by_hour.len(), 24);
    }

    #[test]
    fn get_weekly_stats_empty() {
        let conn = setup();
        let stats = get_weekly_stats(&conn).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn get_heatmap_data_empty() {
        let conn = setup();
        let entries = get_heatmap_data(&conn).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn focus_mins_rounds_to_nearest_minute() {
        let conn = setup();

        // 339 s = 5:39 → rounds up to 6 min (remainder 39 ≥ 30).
        let id1 = insert_session(&conn, "work", 339).unwrap();
        complete_session(&conn, id1, true).unwrap();
        let stats = get_daily_stats(&conn).unwrap();
        assert_eq!(stats.focus_mins, 6, "339 s should round to 6 min");

        // Reset and test round-down: 324 s = 5:24 → rounds down to 5 min (remainder 24 < 30).
        let conn2 = setup();
        let id2 = insert_session(&conn2, "work", 324).unwrap();
        complete_session(&conn2, id2, true).unwrap();
        let stats2 = get_daily_stats(&conn2).unwrap();
        assert_eq!(stats2.focus_mins, 5, "324 s should round to 5 min");

        // Exact minute boundary: 1500 s = 25:00 → stays 25 min.
        let conn3 = setup();
        let id3 = insert_session(&conn3, "work", 1500).unwrap();
        complete_session(&conn3, id3, true).unwrap();
        let stats3 = get_daily_stats(&conn3).unwrap();
        assert_eq!(stats3.focus_mins, 25, "1500 s should be exactly 25 min");
    }

    #[test]
    fn stats_counts_correctly() {
        let conn = setup();

        let id1 = insert_session(&conn, "work", 1500).unwrap();
        complete_session(&conn, id1, true).unwrap();

        let id2 = insert_session(&conn, "work", 1500).unwrap();
        complete_session(&conn, id2, false).unwrap(); // skipped

        let _id3 = insert_session(&conn, "short-break", 300).unwrap();

        let stats = get_all_time_stats(&conn).unwrap();
        assert_eq!(stats.total_work_sessions, 2.0);
        assert_eq!(stats.completed_work_sessions, 1.0);
        assert_eq!(stats.total_work_secs, 1500);
    }
}

// ---------------------------------------------------------------------------
// Import / Export helpers
// ---------------------------------------------------------------------------

/// Returned by `import_sessions` to report how many rows were inserted vs skipped.
#[derive(Debug, Serialize)]
pub struct ImportSummary {
    pub imported: u32,
    pub skipped: u32,
}

/// Export: fetch all non-deleted sessions ordered by `started_at ASC`.
pub fn export_sessions(conn: &Connection) -> Result<Vec<SessionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, started_at, ended_at, round_type, duration_secs, completed,
                subject, subject_topic, study_type, notes, updated_at, deleted_at
         FROM rounds
         WHERE deleted_at IS NULL
         ORDER BY started_at ASC",
    )?;
    let iter = stmt.query_map([], |row| {
        Ok(SessionRow {
            id: row.get(0)?,
            uuid: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            round_type: row.get(4)?,
            duration_secs: row.get(5)?,
            completed: row.get::<_, i64>(6)? != 0,
            subject: row.get(7)?,
            subject_topic: row.get(8)?,
            study_type: row.get(9)?,
            notes: row.get(10)?,
            updated_at: row.get(11)?,
            deleted_at: row.get(12)?,
        })
    })?;
    let mut sessions = Vec::new();
    for row in iter {
        sessions.push(row?);
    }
    Ok(sessions)
}

/// Import: insert `sessions` into the DB, skipping any whose UUID already exists.
/// Returns counts of how many were inserted vs skipped.
pub fn import_sessions(conn: &Connection, sessions: &[SessionRow]) -> Result<ImportSummary> {
    let mut imported = 0u32;
    let mut skipped = 0u32;

    for s in sessions {
        // INSERT OR IGNORE skips the row if the UUID unique constraint fires.
        let affected = conn.execute(
            "INSERT OR IGNORE INTO rounds (
                uuid, started_at, ended_at, round_type, duration_secs, completed,
                subject, subject_topic, study_type, notes, updated_at, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                s.uuid,
                s.started_at,
                s.ended_at,
                s.round_type,
                s.duration_secs,
                s.completed as i64,
                s.subject,
                s.subject_topic,
                s.study_type,
                s.notes,
                s.updated_at,
                s.deleted_at,
            ],
        )?;
        if affected > 0 {
            imported += 1;
        } else {
            skipped += 1;
        }
    }
    Ok(ImportSummary { imported, skipped })
}

// ---------------------------------------------------------------------------
// Insights
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SubjectInsight {
    pub subject: String,
    pub focus_secs: i64,
}

#[derive(Debug, Serialize)]
pub struct InsightsStats {
    pub top_subjects: Vec<SubjectInsight>,
    pub by_day_of_week: Vec<i64>, // 0 = Sunday, 6 = Saturday
    pub by_hour_of_day: Vec<i64>, // 0 = 12am, 23 = 11pm
    pub by_month: Vec<i64>,       // 0 = Jan, 11 = Dec
    pub break_efficiency_percentage: Option<f64>,
    pub schedule_adherence_percentage: Option<f64>,
}

pub fn get_insights_stats(conn: &Connection, filter: &SessionFilter) -> Result<InsightsStats> {
    let mut base_query = String::from("FROM rounds WHERE deleted_at IS NULL AND round_type = 'work' AND completed = 1");
    let mut params = Vec::<rusqlite::types::Value>::new();

    if let Some(subject) = &filter.subject {
        let sql = format!(" AND subject = ?{}", params.len() + 1);
        base_query.push_str(&sql);
        params.push(subject.clone().into());
    }
    if let Some(topic) = &filter.subject_topic {
        let sql = format!(" AND subject_topic = ?{}", params.len() + 1);
        base_query.push_str(&sql);
        params.push(topic.clone().into());
    }
    if let Some(stype) = &filter.study_type {
        let sql = format!(" AND study_type = ?{}", params.len() + 1);
        base_query.push_str(&sql);
        params.push(stype.clone().into());
    }
    if let Some(d_from) = filter.date_from {
        let sql = format!(" AND started_at >= ?{}", params.len() + 1);
        base_query.push_str(&sql);
        params.push(d_from.into());
    }
    if let Some(d_to) = filter.date_to {
        let sql = format!(" AND started_at <= ?{}", params.len() + 1);
        base_query.push_str(&sql);
        params.push(d_to.into());
    }

    // Efficiency & Adherence
    let metrics_query = format!("
        WITH session_metrics AS (
            SELECT 
                ss.id,
                ss.total_pause_secs,
                ss.goal_rounds,
                (IFNULL(ss.ended_at, strftime('%s', 'now')) - ss.started_at) as session_time,
                SUM(CASE WHEN r.round_type != 'work' THEN r.overtime_secs ELSE 0 END) as lazytime,
                SUM(CASE WHEN r.round_type = 'work' THEN r.duration_secs + r.overtime_secs ELSE 0 END) as studying_time,
                SUM(CASE WHEN r.round_type = 'work' AND r.completed = 1 THEN 1 ELSE 0 END) as completed_work_rounds
            FROM study_sessions ss
            JOIN rounds r ON r.study_session_id = ss.id
            WHERE ss.deleted_at IS NULL
            {} -- base_query filters (need to adapt since base_query uses rounds table and completed=1)
            GROUP BY ss.id
        )
        SELECT
            SUM(total_pause_secs) as total_pauses,
            SUM(lazytime) as total_lazytime,
            SUM(session_time) as total_session_time,
            SUM(studying_time) as total_studying_time,
            SUM(CASE WHEN goal_rounds > 0 THEN 1 ELSE 0 END) as total_goal_sessions,
            SUM(CASE WHEN goal_rounds > 0 AND completed_work_rounds >= goal_rounds THEN 1 ELSE 0 END) as adhered_sessions
        FROM session_metrics
    ", base_query.replace("FROM rounds WHERE deleted_at IS NULL AND round_type = 'work' AND completed = 1", "").replace(" started_at", " ss.started_at").replace(" subject", " ss.subject").replace(" study_type", " ss.study_type"));

    let mut break_efficiency_percentage: Option<f64> = None;
    let mut schedule_adherence_percentage: Option<f64> = None;

    if let Ok(mut stmt) = conn.prepare(&metrics_query) {
        if let Ok(mut rows) = stmt.query(rusqlite::params_from_iter(params.iter())) {
            if let Some(row) = rows.next()? {
                let total_pauses: i64 = row.get(0).unwrap_or(0);
                let total_lazytime: i64 = row.get(1).unwrap_or(0);
                let total_session_time: i64 = row.get(2).unwrap_or(0);
                let total_studying_time: i64 = row.get(3).unwrap_or(0);
                let total_goal_sessions: i64 = row.get(4).unwrap_or(0);
                let adhered_sessions: i64 = row.get(5).unwrap_or(0);

                let non_study_time = (total_session_time - total_studying_time).max(0);
                if non_study_time > 0 {
                    let wasted_time = total_pauses + total_lazytime;
                    let efficiency = 1.0 - (wasted_time as f64 / non_study_time as f64);
                    break_efficiency_percentage = Some((efficiency.max(0.0) * 100.0).round());
                } else if total_session_time > 0 {
                    break_efficiency_percentage = Some(100.0);
                }

                if total_goal_sessions > 0 {
                    schedule_adherence_percentage = Some((adhered_sessions as f64 / total_goal_sessions as f64) * 100.0);
                }
            }
        }
    }

    // Top Subjects
    let subject_query = format!("SELECT COALESCE(subject, 'Uncategorized'), SUM(duration_secs) {} GROUP BY COALESCE(subject, 'Uncategorized') ORDER BY SUM(duration_secs) DESC", base_query);
    let mut stmt = conn.prepare(&subject_query)?;
    let top_subjects = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(SubjectInsight {
            subject: row.get(0)?,
            focus_secs: row.get(1)?,
        })
    })?.filter_map(Result::ok).collect();

    // By Day of Week
    let day_query = format!("SELECT CAST(strftime('%w', started_at, 'unixepoch', 'localtime') AS INTEGER), SUM(duration_secs) {} GROUP BY strftime('%w', started_at, 'unixepoch', 'localtime')", base_query);
    let mut stmt = conn.prepare(&day_query)?;
    let mut by_day_of_week = vec![0; 7];
    let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
    while let Some(row) = rows.next()? {
        let day_i: i64 = row.get(0)?;
        let secs: i64 = row.get(1)?;
        let day = day_i as usize;
        if day < 7 {
            by_day_of_week[day] = secs;
        }
    }

    // By Hour of Day
    let hour_query = format!("SELECT CAST(strftime('%H', started_at, 'unixepoch', 'localtime') AS INTEGER), SUM(duration_secs) {} GROUP BY strftime('%H', started_at, 'unixepoch', 'localtime')", base_query);
    let mut stmt = conn.prepare(&hour_query)?;
    let mut by_hour_of_day = vec![0; 24];
    let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
    while let Some(row) = rows.next()? {
        let hour_i: i64 = row.get(0)?;
        let secs: i64 = row.get(1)?;
        let hour = hour_i as usize;
        if hour < 24 {
            by_hour_of_day[hour] = secs;
        }
    }

    // By Month
    let month_query = format!("SELECT CAST(strftime('%m', started_at, 'unixepoch', 'localtime') AS INTEGER), SUM(duration_secs) {} GROUP BY strftime('%m', started_at, 'unixepoch', 'localtime')", base_query);
    let mut stmt = conn.prepare(&month_query)?;
    let mut by_month = vec![0; 12];
    let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
    while let Some(row) = rows.next()? {
        let month_i: i64 = row.get(0)?;
        let secs: i64 = row.get(1)?;
        let month = (month_i - 1) as usize;
        if month < 12 {
            by_month[month] = secs;
        }
    }

    Ok(InsightsStats {
        top_subjects,
        by_day_of_week,
        by_hour_of_day,
        by_month,
        break_efficiency_percentage,
        schedule_adherence_percentage,
    })
}

// ---------------------------------------------------------------------------
// SCHEDULED BLOCKS
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct ScheduledBlock {
    pub id: i64,
    pub subject: String,
    pub day_of_week: i32,
    pub start_minute: i32,
    pub end_minute: i32,
    pub subject_topic: Option<String>,
    pub study_type: Option<String>,
    pub round_tags: Option<String>,
}

pub fn schedule_get_all(conn: &Connection) -> Result<Vec<ScheduledBlock>> {
    let mut stmt = conn.prepare(
        "SELECT id, subject, day_of_week, start_minute, end_minute, subject_topic, study_type, round_tags 
         FROM scheduled_blocks 
         ORDER BY day_of_week ASC, start_minute ASC"
    )?;
    
    let rows = stmt.query_map([], |row| {
        Ok(ScheduledBlock {
            id: row.get(0)?,
            subject: row.get(1)?,
            day_of_week: row.get(2)?,
            start_minute: row.get(3)?,
            end_minute: row.get(4)?,
            subject_topic: row.get(5)?,
            study_type: row.get(6)?,
            round_tags: row.get(7)?,
        })
    })?;

    let mut blocks = Vec::new();
    for b in rows {
        blocks.push(b?);
    }
    Ok(blocks)
}

pub fn schedule_add_block(
    conn: &Connection, 
    subject: &str, 
    day_of_week: i32, 
    start_minute: i32, 
    end_minute: i32,
    subject_topic: Option<&str>,
    study_type: Option<&str>,
    round_tags: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO scheduled_blocks (subject, day_of_week, start_minute, end_minute, created_at, subject_topic, study_type, round_tags) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![subject.trim(), day_of_week, start_minute, end_minute, unix_now(), subject_topic.map(|s| s.trim()), study_type.map(|s| s.trim()), round_tags],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn schedule_delete_block(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM scheduled_blocks WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn schedule_update_block(
    conn: &Connection, 
    id: i64, 
    day_of_week: i32, 
    start_minute: i32, 
    end_minute: i32,
    subject_topic: Option<&str>,
    study_type: Option<&str>,
    round_tags: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE scheduled_blocks 
         SET day_of_week = ?1, start_minute = ?2, end_minute = ?3, subject_topic = ?4, study_type = ?5, round_tags = ?6 
         WHERE id = ?7",
        params![day_of_week, start_minute, end_minute, subject_topic.map(|s| s.trim()), study_type.map(|s| s.trim()), round_tags, id],
    )?;
    Ok(())
}
