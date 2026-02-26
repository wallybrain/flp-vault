use crate::services::scanner;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn scan_folder(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let db = Arc::clone(&state.db);

    let running_flag = {
        let status = state.scan_status.lock().unwrap();
        {
            let mut r = status.running.lock().unwrap();
            *r = true;
        }
        Arc::clone(&status.running)
    };

    std::thread::spawn(move || {
        scanner::run_scan(&path, db, app_handle, running_flag);
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    let status = state.scan_status.lock().unwrap();
    let mut running = status.running.lock().unwrap();
    *running = false;
    Ok(())
}
