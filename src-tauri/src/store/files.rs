use crate::parser::types::FlpMetadata;
use rusqlite::Connection;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub hash: String,
    pub path: String,
    pub file_size: i64,
    pub mtime: i64,
    pub bpm: Option<f64>,
    pub channel_count: Option<i64>,
    pub plugins_json: Option<String>,
    pub fl_version: Option<String>,
}

pub fn is_cached(db: &Mutex<Connection>, path: &str, file_size: i64, mtime: i64) -> bool {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT 1 FROM path_index WHERE path = ?1 AND file_size = ?2 AND mtime = ?3",
        [path, &file_size.to_string(), &mtime.to_string()],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

pub fn hash_in_cache(db: &Mutex<Connection>, hash: &str) -> bool {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT 1 FROM files WHERE hash = ?1",
        [hash],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

pub fn update_path_index(
    db: &Mutex<Connection>,
    path: &str,
    hash: &str,
    file_size: i64,
    mtime: i64,
) {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO path_index (path, hash, file_size, mtime) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(path) DO UPDATE SET hash = excluded.hash, file_size = excluded.file_size, mtime = excluded.mtime",
        rusqlite::params![path, hash, file_size, mtime],
    )
    .unwrap();
}

pub fn upsert_file(
    db: &Mutex<Connection>,
    hash: &str,
    path: &str,
    file_size: i64,
    mtime: i64,
    meta: &FlpMetadata,
) {
    let plugins_json = {
        let generators: Vec<String> = meta
            .generators
            .iter()
            .filter_map(|g| g.plugin_name.clone().or_else(|| Some(g.name.clone())))
            .collect();
        let mut all_plugins = generators;
        all_plugins.extend(meta.effects.clone());
        serde_json::to_string(&all_plugins).unwrap_or_else(|_| "[]".to_string())
    };

    let warnings_json =
        serde_json::to_string(&meta.warnings).unwrap_or_else(|_| "[]".to_string());

    let parsed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO files (hash, path, file_size, mtime, bpm, time_sig_num, time_sig_den,
                                channel_count, pattern_count, mixer_track_count, plugins_json,
                                warnings_json, fl_version, parsed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(hash) DO UPDATE SET
                path = excluded.path,
                file_size = excluded.file_size,
                mtime = excluded.mtime,
                bpm = excluded.bpm,
                time_sig_num = excluded.time_sig_num,
                time_sig_den = excluded.time_sig_den,
                channel_count = excluded.channel_count,
                pattern_count = excluded.pattern_count,
                mixer_track_count = excluded.mixer_track_count,
                plugins_json = excluded.plugins_json,
                warnings_json = excluded.warnings_json,
                fl_version = excluded.fl_version,
                parsed_at = excluded.parsed_at",
            rusqlite::params![
                hash,
                path,
                file_size,
                mtime,
                meta.bpm.map(|b| b as f64),
                meta.time_sig_num.map(|n| n as i64),
                meta.time_sig_den.map(|d| d as i64),
                meta.channel_count as i64,
                meta.pattern_count as i64,
                meta.mixer_track_count as i64,
                plugins_json,
                warnings_json,
                meta.fl_version,
                parsed_at,
            ],
        )
        .unwrap();
    }

    update_path_index(db, path, hash, file_size, mtime);
}

pub fn list_all_files(db: &Mutex<Connection>) -> Vec<FileRecord> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT f.hash, f.path, f.file_size, f.mtime, f.bpm, f.channel_count,
                    f.plugins_json, f.fl_version
             FROM files f
             ORDER BY f.path ASC",
        )
        .unwrap();

    stmt.query_map([], |row| {
        Ok(FileRecord {
            hash: row.get(0)?,
            path: row.get(1)?,
            file_size: row.get(2)?,
            mtime: row.get(3)?,
            bpm: row.get(4)?,
            channel_count: row.get(5)?,
            plugins_json: row.get(6)?,
            fl_version: row.get(7)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}
