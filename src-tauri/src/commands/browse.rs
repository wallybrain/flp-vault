use crate::state::AppState;
use crate::store::files::{list_all_files, FileRecord};
use tauri::State;

#[tauri::command]
pub fn list_scanned_files(state: State<'_, AppState>) -> Result<Vec<FileRecord>, String> {
    Ok(list_all_files(&state.db))
}
