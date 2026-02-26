// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod matcher;
mod parser;
mod services;
mod state;
mod store;

use commands::{
    cancel_scan, confirm_groups, get_settings, list_groups, list_scanned_files, propose_groups,
    reset_groups, save_settings, scan_folder,
};
use state::AppState;
use store::connection::init_db;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            let db_mutex = init_db(&app_data_dir)
                .expect("Failed to initialize database");

            let db = db_mutex
                .into_inner()
                .expect("Failed to unwrap DB Mutex");

            app.manage(AppState::new(db));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            cancel_scan,
            get_settings,
            save_settings,
            list_scanned_files,
            propose_groups,
            confirm_groups,
            list_groups,
            reset_groups,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
