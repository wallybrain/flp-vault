use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct ScanStatus {
    pub total: usize,
    pub done: usize,
    pub running: bool,
}

impl ScanStatus {
    pub fn new() -> Self {
        Self {
            total: 0,
            done: 0,
            running: false,
        }
    }
}

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub scan_status: Mutex<ScanStatus>,
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            scan_status: Mutex::new(ScanStatus::new()),
        }
    }
}
