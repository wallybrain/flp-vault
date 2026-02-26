use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS files (
            hash              TEXT PRIMARY KEY,
            path              TEXT NOT NULL,
            file_size         INTEGER NOT NULL,
            mtime             INTEGER NOT NULL,
            bpm               REAL,
            time_sig_num      INTEGER,
            time_sig_den      INTEGER,
            channel_count     INTEGER,
            pattern_count     INTEGER,
            mixer_track_count INTEGER,
            plugins_json      TEXT,
            warnings_json     TEXT,
            fl_version        TEXT,
            parsed_at         INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS path_index (
            path      TEXT PRIMARY KEY,
            hash      TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            mtime     INTEGER NOT NULL,
            FOREIGN KEY (hash) REFERENCES files(hash)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS song_groups (
            group_id       TEXT PRIMARY KEY,
            canonical_name TEXT NOT NULL,
            confirmed_at   INTEGER NOT NULL,
            is_ignored     INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS group_files (
            hash              TEXT NOT NULL,
            group_id          TEXT NOT NULL,
            is_ignored        INTEGER NOT NULL DEFAULT 0,
            manually_assigned INTEGER NOT NULL DEFAULT 0,
            assigned_at       INTEGER NOT NULL,
            PRIMARY KEY (hash, group_id),
            FOREIGN KEY (hash) REFERENCES files(hash),
            FOREIGN KEY (group_id) REFERENCES song_groups(group_id)
        );
        ",
    )?;
    Ok(())
}
