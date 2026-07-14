use rusqlite::{Connection, Result};

/// Full schema for version 1. Tables use IF NOT EXISTS so the batch is
/// idempotent, but the schema_version check in `run()` prevents re-execution.
const MIGRATION_1: &str = "
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS settings (
        key   TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS sessions (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        started_at    INTEGER NOT NULL,
        ended_at      INTEGER,
        round_type    TEXT NOT NULL CHECK(round_type IN ('work', 'short-break', 'long-break')),
        duration_secs INTEGER NOT NULL CHECK(duration_secs > 0),
        completed     INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1))
    );

    CREATE TABLE IF NOT EXISTS custom_themes (
        id     INTEGER PRIMARY KEY AUTOINCREMENT,
        name   TEXT NOT NULL UNIQUE,
        colors TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
    CREATE INDEX IF NOT EXISTS idx_sessions_round_type ON sessions(round_type);

    INSERT INTO schema_version VALUES (1);
";

/// Migrates timer duration storage from minute-resolution keys to second-resolution keys.
/// Reads existing `time_*_mins` rows, multiplies by 60, writes `time_*_secs`, then deletes
/// the old keys so key names align with the Settings struct field names.
const MIGRATION_2: &str = "
    INSERT OR IGNORE INTO settings (key, value)
        SELECT 'time_work_secs', CAST(CAST(value AS INTEGER) * 60 AS TEXT)
          FROM settings WHERE key = 'time_work_mins';
    INSERT OR IGNORE INTO settings (key, value)
        SELECT 'time_short_break_secs', CAST(CAST(value AS INTEGER) * 60 AS TEXT)
          FROM settings WHERE key = 'time_short_break_mins';
    INSERT OR IGNORE INTO settings (key, value)
        SELECT 'time_long_break_secs', CAST(CAST(value AS INTEGER) * 60 AS TEXT)
          FROM settings WHERE key = 'time_long_break_mins';
    DELETE FROM settings WHERE key IN
        ('time_work_mins', 'time_short_break_mins', 'time_long_break_mins');
    INSERT INTO schema_version VALUES (2);
";

/// Seeds the `check_for_updates` setting for users upgrading from a version
/// that did not have this setting. Fresh installs get it via seed_defaults.
const MIGRATION_3: &str = "
    INSERT OR IGNORE INTO settings (key, value) VALUES ('check_for_updates', 'true');
    INSERT INTO schema_version VALUES (3);
";

/// Seeds the `global_shortcuts_enabled` setting for all installs. Defaults to
/// 'false' — global shortcuts are now opt-in. This is a breaking change for
/// existing users who relied on shortcuts being active by default; they must
/// re-enable them in Settings → Shortcuts.
const MIGRATION_4: &str = "
    INSERT OR IGNORE INTO settings (key, value) VALUES ('global_shortcuts_enabled', 'false');
    INSERT INTO schema_version VALUES (4);
";

/// Seeds the `short_breaks_enabled` and `long_breaks_enabled` settings for
/// users upgrading from a version that did not have these settings.
/// Both default to 'true' — existing behaviour is preserved.
const MIGRATION_5: &str = "
    INSERT OR IGNORE INTO settings (key, value) VALUES ('short_breaks_enabled', 'true');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('long_breaks_enabled', 'true');
    INSERT INTO schema_version VALUES (5);
";

/// Seeds the seven local shortcut key bindings for users upgrading from a version
/// that did not have this feature. These shortcuts are handled entirely by the frontend
/// (keydown listeners) and require no Rust-side dispatch logic.
const MIGRATION_6: &str = "
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_toggle', ' ');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_reset', 'ArrowLeft');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_skip', 'ArrowRight');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_volume_down', 'ArrowDown');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_volume_up', 'ArrowUp');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_mute', 'm');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_fullscreen', 'F11');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_palette', 'p');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('local_shortcut_stats', 's');
    INSERT INTO schema_version VALUES (6);
";

/// Recreates the `sessions` table from scratch (losing existing session data, per user request)
/// to include new fields: uuid, subject, subject_topic, study_type, notes, updated_at, deleted_at.
const MIGRATION_7: &str = "
    DROP TABLE IF EXISTS sessions;

    CREATE TABLE sessions (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        uuid          TEXT NOT NULL UNIQUE,
        started_at    INTEGER NOT NULL,
        ended_at      INTEGER,
        round_type    TEXT NOT NULL CHECK(round_type IN ('work', 'short-break', 'long-break')),
        duration_secs INTEGER NOT NULL CHECK(duration_secs > 0),
        completed     INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1)),
        subject       TEXT,
        subject_topic TEXT,
        study_type    TEXT,
        notes         TEXT,
        updated_at    INTEGER,
        deleted_at    INTEGER
    );

    CREATE INDEX idx_sessions_started_at ON sessions(started_at);
    CREATE INDEX idx_sessions_round_type ON sessions(round_type);

    INSERT INTO schema_version VALUES (7);
";

/// Create subjects table for independent subject tracking
const MIGRATION_8: &str = "
    CREATE TABLE IF NOT EXISTS subjects (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        name       TEXT NOT NULL UNIQUE,
        color      TEXT,
        created_at INTEGER NOT NULL
    );
    
    INSERT OR IGNORE INTO subjects (name, created_at)
        SELECT DISTINCT subject, CAST(strftime('%s', 'now') AS INTEGER)
        FROM sessions 
        WHERE subject IS NOT NULL AND subject != '';

    INSERT INTO schema_version VALUES (8);
";

/// Add weekly_goal column to subjects table
const MIGRATION_9: &str = "
    ALTER TABLE subjects ADD COLUMN weekly_goal INTEGER;
    
    INSERT INTO schema_version VALUES (9);
";

/// Create scheduled_blocks table
const MIGRATION_10: &str = "
    CREATE TABLE scheduled_blocks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        subject TEXT NOT NULL,
        day_of_week INTEGER NOT NULL,
        start_minute INTEGER NOT NULL,
        end_minute INTEGER NOT NULL,
        created_at INTEGER NOT NULL
    );
    
    INSERT INTO schema_version VALUES (10);
";

/// Sessions refactor: Introduce study_sessions and rename sessions to rounds
const MIGRATION_11: &str = "
    CREATE TABLE study_sessions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        uuid TEXT NOT NULL UNIQUE,
        started_at INTEGER NOT NULL,
        ended_at INTEGER,
        goal_rounds INTEGER NOT NULL DEFAULT 0,
        total_pause_secs INTEGER NOT NULL DEFAULT 0,
        subject TEXT,
        subject_topic TEXT,
        study_type TEXT,
        notes TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER,
        deleted_at INTEGER
    );

    ALTER TABLE sessions RENAME TO rounds;
    ALTER TABLE rounds ADD COLUMN study_session_id INTEGER REFERENCES study_sessions(id);
    ALTER TABLE rounds ADD COLUMN pause_secs INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE rounds ADD COLUMN overtime_secs INTEGER NOT NULL DEFAULT 0;
    DROP INDEX IF EXISTS idx_sessions_started_at;
    DROP INDEX IF EXISTS idx_sessions_round_type;
    CREATE INDEX idx_rounds_started_at ON rounds(started_at);
    CREATE INDEX idx_rounds_round_type ON rounds(round_type);

    INSERT INTO study_sessions (uuid, started_at, ended_at, goal_rounds, total_pause_secs, subject, subject_topic, study_type, created_at)
    SELECT 
      lower(hex(randomblob(16))) as uuid,
      MIN(started_at) as started_at,
      MAX(ended_at) as ended_at,
      0 as goal_rounds,
      0 as total_pause_secs,
      subject,
      subject_topic,
      study_type,
      MIN(started_at) as created_at
    FROM rounds
    WHERE deleted_at IS NULL
    GROUP BY date(started_at, 'unixepoch', 'localtime'), subject, subject_topic, study_type;

    UPDATE rounds
    SET study_session_id = (
      SELECT id FROM study_sessions s 
      WHERE date(rounds.started_at, 'unixepoch', 'localtime') = date(s.started_at, 'unixepoch', 'localtime')
        AND IFNULL(rounds.subject, '') = IFNULL(s.subject, '')
        AND IFNULL(rounds.subject_topic, '') = IFNULL(s.subject_topic, '')
        AND IFNULL(rounds.study_type, '') = IFNULL(s.study_type, '')
    )
    WHERE deleted_at IS NULL;

    INSERT INTO schema_version VALUES (11);
";

const MIGRATION_12: &str = "
    ALTER TABLE scheduled_blocks ADD COLUMN subject_topic TEXT;
    ALTER TABLE scheduled_blocks ADD COLUMN study_type TEXT;
    ALTER TABLE scheduled_blocks ADD COLUMN round_tags TEXT;

    INSERT INTO schema_version VALUES (12);
";

/// Fix study_session_id for historical break rounds (migration 11 artifact)
const MIGRATION_13: &str = "
    UPDATE rounds
    SET study_session_id = (
        SELECT study_session_id
        FROM rounds AS r2
        WHERE r2.round_type = 'work'
          AND r2.started_at <= rounds.started_at
          AND (rounds.started_at - r2.started_at) < 43200
          AND r2.deleted_at IS NULL
        ORDER BY r2.started_at DESC
        LIMIT 1
    )
    WHERE round_type IN ('short-break', 'long-break')
      AND deleted_at IS NULL;

    DELETE FROM study_sessions
    WHERE id NOT IN (SELECT DISTINCT study_session_id FROM rounds WHERE study_session_id IS NOT NULL);

    UPDATE study_sessions
    SET 
      started_at = (SELECT MIN(started_at) FROM rounds WHERE study_session_id = study_sessions.id AND deleted_at IS NULL),
      ended_at = (SELECT MAX(ended_at) FROM rounds WHERE study_session_id = study_sessions.id AND deleted_at IS NULL)
    WHERE id IN (SELECT DISTINCT study_session_id FROM rounds WHERE study_session_id IS NOT NULL);

    INSERT INTO schema_version VALUES (13);
";
const MIGRATION_15: &str = "
    ALTER TABLE rounds ADD COLUMN is_half_session INTEGER NOT NULL DEFAULT 0;
    
    INSERT INTO schema_version VALUES (15);
";

const MIGRATION_16: &str = "
    ALTER TABLE rounds ADD COLUMN exclude_from_stats INTEGER NOT NULL DEFAULT 0;
    
    INSERT INTO schema_version VALUES (16);
";

const MIGRATION_17: &str = "
    INSERT INTO study_sessions (
        uuid, started_at, ended_at, goal_rounds, total_pause_secs, subject, subject_topic, study_type, notes, created_at, updated_at
    )
    SELECT
        lower(hex(randomblob(16))) as uuid,
        started_at,
        ended_at,
        1 as goal_rounds,
        0 as total_pause_secs,
        subject,
        subject_topic,
        study_type,
        notes,
        started_at as created_at,
        updated_at
    FROM rounds
    WHERE study_session_id IS NULL AND deleted_at IS NULL;

    UPDATE rounds
    SET study_session_id = (
        SELECT id FROM study_sessions
        WHERE study_sessions.started_at = rounds.started_at
        ORDER BY id DESC
        LIMIT 1
    )
    WHERE study_session_id IS NULL AND deleted_at IS NULL;

    INSERT INTO schema_version VALUES (17);
";

/// Apply any pending migrations. Each migration is wrapped in a transaction
/// so a partial failure leaves the database unchanged.
pub fn run(conn: &Connection) -> Result<()> {
    let version = current_version(conn)?;


    if version < 1 {
        log::info!("[db/migrations] applying MIGRATION_1: initial schema");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_1} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_1 complete");
    }

    if version < 2 {
        log::info!("[db/migrations] applying MIGRATION_2: timer durations minutes → seconds");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_2} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_2 complete");
    }

    if version < 3 {
        log::info!("[db/migrations] applying MIGRATION_3: seed check_for_updates setting");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_3} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_3 complete");
    }

    if version < 4 {
        log::info!("[db/migrations] applying MIGRATION_4: seed global_shortcuts_enabled setting");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_4} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_4 complete");
    }

    if version < 5 {
        log::info!("[db/migrations] applying MIGRATION_5: seed short_breaks_enabled and long_breaks_enabled");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_5} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_5 complete");
    }

    if version < 6 {
        log::info!("[db/migrations] applying MIGRATION_6: seed local shortcut key bindings");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_6} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_6 complete");
    }

    if version < 7 {
        log::info!("[db/migrations] applying MIGRATION_7: recreate sessions table with extended schema");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_7} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_7 complete");
    }

    if version < 8 {
        log::info!("[db/migrations] applying MIGRATION_8: create subjects table");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_8} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_8 complete");
    }

    if version < 9 {
        log::info!("[db/migrations] applying MIGRATION_9: add weekly_goal to subjects");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_9} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_9 complete");
    }

    if version < 10 {
        log::info!("[db/migrations] applying MIGRATION_10: create scheduled_blocks table");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_10} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_10 complete");
    }

    if version < 11 {
        log::info!("[db/migrations] applying MIGRATION_11: group rounds into study_sessions");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_11} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_11 complete");
    }

    if version < 12 {
        log::info!("[db/migrations] applying MIGRATION_12: add scheduled blocks tags");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_12} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_12 complete");
    }

    if version < 13 {
        log::info!("[db/migrations] applying MIGRATION_13: fix historical break rounds");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_13} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_13 complete");
    }

    if version < 14 {
        log::info!("[db/migrations] applying MIGRATION_14: rebuild study sessions");
        conn.execute_batch("BEGIN;")?;

        conn.execute("UPDATE rounds SET study_session_id = NULL", [])?;
        conn.execute("DELETE FROM study_sessions", [])?;

        struct RoundRow {
            id: i64,
            started_at: i64,
            ended_at: Option<i64>,
            round_type: String,
            subject: Option<String>,
            subject_topic: Option<String>,
            study_type: Option<String>,
        }

        let mut rounds = Vec::new();
        {
            let mut stmt = conn.prepare("SELECT id, started_at, ended_at, round_type, subject, subject_topic, study_type FROM rounds WHERE deleted_at IS NULL ORDER BY started_at ASC")?;
            let r_iter = stmt.query_map([], |row| {
                Ok(RoundRow {
                    id: row.get(0)?,
                    started_at: row.get(1)?,
                    ended_at: row.get(2)?,
                    round_type: row.get(3)?,
                    subject: row.get(4)?,
                    subject_topic: row.get(5)?,
                    study_type: row.get(6)?,
                })
            })?;
            for r in r_iter {
                rounds.push(r?);
            }
        }

        let mut current_session_id: Option<i64> = None;
        let mut last_ended_at: i64 = 0;
        let mut current_subject: Option<String> = None;
        let mut current_topic: Option<String> = None;
        let mut current_type: Option<String> = None;

        for r in rounds {
            let mut is_new_session = false;

            if current_session_id.is_none() {
                is_new_session = true;
            } else if (r.started_at - last_ended_at) > 7200 {
                is_new_session = true;
            } else if r.round_type == "work" {
                if r.subject != current_subject || r.subject_topic != current_topic || r.study_type != current_type {
                    is_new_session = true;
                }
            }

            if is_new_session {
                let subject_val = if r.round_type == "work" { r.subject.clone() } else { None };
                let topic_val = if r.round_type == "work" { r.subject_topic.clone() } else { None };
                let type_val = if r.round_type == "work" { r.study_type.clone() } else { None };

                conn.execute(
                    "INSERT INTO study_sessions (uuid, started_at, ended_at, goal_rounds, total_pause_secs, subject, subject_topic, study_type, created_at)
                     VALUES (lower(hex(randomblob(16))), ?1, ?2, 0, 0, ?3, ?4, ?5, ?6)",
                    rusqlite::params![r.started_at, r.ended_at.unwrap_or(r.started_at), subject_val, topic_val, type_val, r.started_at],
                )?;
                current_session_id = Some(conn.last_insert_rowid());
                current_subject = subject_val;
                current_topic = topic_val;
                current_type = type_val;
            } else {
                if let Some(sid) = current_session_id {
                    let end_time = r.ended_at.unwrap_or(r.started_at);
                    conn.execute("UPDATE study_sessions SET ended_at = MAX(IFNULL(ended_at, 0), ?1) WHERE id = ?2", rusqlite::params![end_time, sid])?;
                }
            }

            if let Some(sid) = current_session_id {
                conn.execute("UPDATE rounds SET study_session_id = ?1 WHERE id = ?2", rusqlite::params![sid, r.id])?;
            }

            last_ended_at = r.ended_at.unwrap_or(r.started_at);
        }

        conn.execute("INSERT INTO schema_version VALUES (14)", [])?;
        conn.execute_batch("COMMIT;")?;
        log::info!("[db/migrations] MIGRATION_14 complete");
    }

    if version < 15 {
        log::info!("[db/migrations] applying MIGRATION_15: add is_half_session to rounds");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_15} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_15 complete");
    }

    if version < 16 {
        log::info!("[db/migrations] applying MIGRATION_16: add exclude_from_stats to rounds");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_16} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_16 complete");
    }

    if version < 17 {
        log::info!("[db/migrations] applying MIGRATION_17: fix orphaned manual rounds");
        conn.execute_batch(&format!("BEGIN; {MIGRATION_17} COMMIT;"))?;
        log::info!("[db/migrations] MIGRATION_17 complete");
    }

    Ok(())
}

/// Returns the current schema version, or 0 if the database is fresh.
fn current_version(conn: &Connection) -> Result<i64> {
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        return Ok(0);
    }

    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        // Second run must not error (version check prevents re-application).
        run(&conn).unwrap();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 16);
    }

    #[test]
    fn all_tables_created() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        for table in &["settings", "rounds", "custom_themes", "schema_version", "subjects", "study_sessions"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table '{table}' was not created");
        }
    }
}
