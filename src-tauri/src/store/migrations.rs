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
        ",
    )?;
    Ok(())
}
