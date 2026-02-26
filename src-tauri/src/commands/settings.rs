use crate::state::AppState;
use crate::store::settings::{get_all_settings, set_setting, Settings};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsValidation {
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(get_all_settings(&state.db))
}

#[tauri::command]
pub fn save_settings(
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<SettingsValidation, String> {
    let mut warnings = Vec::new();

    // Validate folder paths
    if !settings.source_folder.is_empty() && !Path::new(&settings.source_folder).exists() {
        warnings.push(format!(
            "Source folder does not exist: {}",
            settings.source_folder
        ));
    }
    if !settings.organized_folder.is_empty() && !Path::new(&settings.organized_folder).exists() {
        warnings.push(format!(
            "Organized folder does not exist: {}",
            settings.organized_folder
        ));
    }
    if !settings.originals_folder.is_empty() && !Path::new(&settings.originals_folder).exists() {
        warnings.push(format!(
            "Originals folder does not exist: {}",
            settings.originals_folder
        ));
    }

    // Warn on conflicting paths
    if !settings.source_folder.is_empty()
        && !settings.organized_folder.is_empty()
        && settings.source_folder == settings.organized_folder
    {
        warnings.push("Source folder and Organized folder are the same path.".to_string());
    }
    if !settings.source_folder.is_empty()
        && !settings.originals_folder.is_empty()
        && settings.source_folder == settings.originals_folder
    {
        warnings.push("Source folder and Originals folder are the same path.".to_string());
    }

    // Persist settings
    set_setting(&state.db, "source_folder", &settings.source_folder);
    set_setting(&state.db, "organized_folder", &settings.organized_folder);
    set_setting(&state.db, "originals_folder", &settings.originals_folder);

    Ok(SettingsValidation { warnings })
}
