use crate::matcher::ProposedGroup;
use crate::services::grouper;
use crate::state::AppState;
use crate::store::{groups, settings};
use crate::store::groups::GroupConfirmation;
use tauri::State;

#[tauri::command]
pub fn propose_groups(state: State<'_, AppState>) -> Result<Vec<ProposedGroup>, String> {
    let threshold = settings::get_setting(&state.db, "grouping_threshold")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.65);
    Ok(grouper::run_grouper(&state.db, threshold))
}

#[tauri::command]
pub fn confirm_groups(
    groups_input: Vec<GroupConfirmation>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    groups::confirm_groups(&state.db, &groups_input)
}

#[tauri::command]
pub fn list_groups(state: State<'_, AppState>) -> Result<Vec<groups::ConfirmedGroup>, String> {
    Ok(groups::list_confirmed_groups(&state.db))
}

#[tauri::command]
pub fn reset_groups(state: State<'_, AppState>) -> Result<(), String> {
    groups::clear_all_groups(&state.db)
}
