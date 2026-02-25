use crate::store::migrations::run_migrations;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub fn init_db(app_data_dir: &Path) -> Result<Mutex<Connection>, Box<dyn std::error::Error>> {
    // CRITICAL: Create the directory before opening the DB — Tauri does not do this automatically
    std::fs::create_dir_all(app_data_dir)?;

    let db_path = app_data_dir.join("flp-vault.db");
    let conn = Connection::open(&db_path)?;

    // Performance and safety pragmas
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;
        PRAGMA synchronous=NORMAL;
        ",
    )?;

    run_migrations(&conn)?;

    Ok(Mutex::new(conn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::settings::{get_all_settings, get_setting, set_setting};

    #[test]
    fn test_init_db_creates_tables() {
        let dir = tempfile::tempdir().unwrap();
        let db_mutex = init_db(dir.path()).unwrap();
        let db = db_mutex.lock().unwrap();

        let tables: Vec<String> = {
            let mut stmt = db
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };

        assert!(tables.contains(&"files".to_string()), "files table missing");
        assert!(
            tables.contains(&"path_index".to_string()),
            "path_index table missing"
        );
        assert!(
            tables.contains(&"settings".to_string()),
            "settings table missing"
        );
    }

    #[test]
    fn test_settings_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db_mutex = init_db(dir.path()).unwrap();
        let settings = get_all_settings(&db_mutex);

        assert!(
            !settings.source_folder.is_empty(),
            "source_folder default should be non-empty"
        );
        assert!(
            !settings.organized_folder.is_empty(),
            "organized_folder default should be non-empty"
        );
        assert!(
            !settings.originals_folder.is_empty(),
            "originals_folder default should be non-empty"
        );
    }

    #[test]
    fn test_set_and_get_setting() {
        let dir = tempfile::tempdir().unwrap();
        let db_mutex = init_db(dir.path()).unwrap();

        set_setting(&db_mutex, "test_key", "test_value");
        let result = get_setting(&db_mutex, "test_key");

        assert_eq!(result, Some("test_value".to_string()));
    }
}
