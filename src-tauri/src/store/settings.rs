use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub source_folder: String,
    pub organized_folder: String,
    pub originals_folder: String,
}

fn default_source_folder() -> String {
    // FL Studio default project path: Documents\Image-Line\FL Studio\Projects
    // Fall back to home dir if documents dir is unavailable (e.g. in test environments)
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("Image-Line")
        .join("FL Studio")
        .join("Projects")
        .to_string_lossy()
        .into_owned()
}

fn default_organized_folder() -> String {
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("FLP Vault").to_string_lossy().into_owned()
}

fn default_originals_folder() -> String {
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("FLP Vault Originals")
        .to_string_lossy()
        .into_owned()
}

pub fn get_setting(db: &Mutex<Connection>, key: &str) -> Option<String> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .ok()
}

pub fn set_setting(db: &Mutex<Connection>, key: &str, value: &str) {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .unwrap();
}

pub fn get_all_settings(db: &Mutex<Connection>) -> Settings {
    let source_folder = get_setting(db, "source_folder")
        .unwrap_or_else(default_source_folder);
    let organized_folder = get_setting(db, "organized_folder")
        .unwrap_or_else(default_organized_folder);
    let originals_folder = get_setting(db, "originals_folder")
        .unwrap_or_else(default_originals_folder);

    Settings {
        source_folder,
        organized_folder,
        originals_folder,
    }
}
