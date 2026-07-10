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
        assert_eq!(v, 12);
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
