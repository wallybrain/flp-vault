use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmedGroup {
    pub group_id: String,
    pub canonical_name: String,
    pub file_hashes: Vec<String>,
    pub ignored_hashes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupConfirmation {
    pub canonical_name: String,
    pub file_hashes: Vec<String>,
    pub ignored_hashes: Vec<String>,
}

pub fn confirm_groups(db: &Mutex<Connection>, groups: &[GroupConfirmation]) -> Result<(), String> {
    let conn = db.lock().unwrap();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    let now = Utc::now().timestamp();

    for group in groups {
        let group_id = Uuid::new_v4().to_string();

        tx.execute(
            "INSERT INTO song_groups (group_id, canonical_name, confirmed_at, is_ignored) VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![group_id, group.canonical_name, now],
        )
        .map_err(|e| e.to_string())?;

        for hash in &group.file_hashes {
            let is_ignored = if group.ignored_hashes.contains(hash) { 1 } else { 0 };
            tx.execute(
                "INSERT INTO group_files (hash, group_id, is_ignored, manually_assigned, assigned_at) VALUES (?1, ?2, ?3, 0, ?4)",
                rusqlite::params![hash, group_id, is_ignored, now],
            )
            .map_err(|e| e.to_string())?;
        }

        // Insert any ignored hashes not already in file_hashes
        for hash in &group.ignored_hashes {
            if !group.file_hashes.contains(hash) {
                tx.execute(
                    "INSERT OR IGNORE INTO group_files (hash, group_id, is_ignored, manually_assigned, assigned_at) VALUES (?1, ?2, 1, 0, ?3)",
                    rusqlite::params![hash, group_id, now],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_confirmed_groups(db: &Mutex<Connection>) -> Vec<ConfirmedGroup> {
    let conn = db.lock().unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT sg.group_id, sg.canonical_name, gf.hash, gf.is_ignored
             FROM song_groups sg
             JOIN group_files gf ON sg.group_id = gf.group_id
             ORDER BY sg.canonical_name, sg.group_id, gf.hash",
        )
        .expect("Failed to prepare list_confirmed_groups query");

    // group_id -> (canonical_name, file_hashes, ignored_hashes)
    let mut group_map: BTreeMap<String, (String, Vec<String>, Vec<String>)> = BTreeMap::new();

    let rows: Vec<(String, String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .expect("Failed to query group_files")
        .flatten()
        .collect();

    for (group_id, canonical_name, hash, is_ignored) in rows {
        let entry = group_map
            .entry(group_id)
            .or_insert_with(|| (canonical_name, Vec::new(), Vec::new()));
        if is_ignored == 1 {
            entry.2.push(hash);
        } else {
            entry.1.push(hash);
        }
    }

    group_map
        .into_iter()
        .map(|(group_id, (canonical_name, file_hashes, ignored_hashes))| ConfirmedGroup {
            group_id,
            canonical_name,
            file_hashes,
            ignored_hashes,
        })
        .collect()
}

pub fn get_group_for_file(db: &Mutex<Connection>, hash: &str) -> Option<String> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT group_id FROM group_files WHERE hash = ?1 AND is_ignored = 0 LIMIT 1",
        [hash],
        |row| row.get(0),
    )
    .ok()
}

pub fn has_confirmed_groups(db: &Mutex<Connection>) -> bool {
    let conn = db.lock().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM song_groups", [], |row| row.get(0))
        .unwrap_or(0);
    count > 0
}

pub fn mark_file_ignored(db: &Mutex<Connection>, hash: &str) -> Result<(), String> {
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE group_files SET is_ignored = 1 WHERE hash = ?1",
        [hash],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_all_groups(db: &Mutex<Connection>) -> Result<(), String> {
    let conn = db.lock().unwrap();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM group_files", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM song_groups", [])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::connection::init_db;
    use tempfile::tempdir;

    #[test]
    fn test_confirm_and_list_groups() {
        let dir = tempdir().unwrap();
        let db = init_db(dir.path()).unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO files (hash, path, file_size, mtime, parsed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                ["abc123", "/test.flp", "1000", "1700000000", "1700000000"],
            )
            .unwrap();
        }
        let groups = vec![GroupConfirmation {
            canonical_name: "Test Song".to_string(),
            file_hashes: vec!["abc123".to_string()],
            ignored_hashes: vec![],
        }];
        confirm_groups(&db, &groups).unwrap();
        let confirmed = list_confirmed_groups(&db);
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].canonical_name, "Test Song");
        assert_eq!(confirmed[0].file_hashes.len(), 1);
    }

    #[test]
    fn test_has_confirmed_groups() {
        let dir = tempdir().unwrap();
        let db = init_db(dir.path()).unwrap();
        assert!(!has_confirmed_groups(&db));
    }

    #[test]
    fn test_clear_all_groups() {
        let dir = tempdir().unwrap();
        let db = init_db(dir.path()).unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO files (hash, path, file_size, mtime, parsed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                ["abc123", "/test.flp", "1000", "1700000000", "1700000000"],
            )
            .unwrap();
        }
        let groups = vec![GroupConfirmation {
            canonical_name: "Test".to_string(),
            file_hashes: vec!["abc123".to_string()],
            ignored_hashes: vec![],
        }];
        confirm_groups(&db, &groups).unwrap();
        assert!(has_confirmed_groups(&db));
        clear_all_groups(&db).unwrap();
        assert!(!has_confirmed_groups(&db));
    }
}
