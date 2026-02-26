import { getSettings, scanFolder } from './api.js';
import * as scanTable from './panels/scan-table.js';
import * as settingsPanel from './panels/settings-panel.js';

document.addEventListener('DOMContentLoaded', async () => {
    const mainContent = document.getElementById('main-content');
    const emptyState = document.getElementById('empty-state');
    const btnSettings = document.getElementById('btn-settings');
    const btnOpenSettings = document.getElementById('btn-open-settings');
    const settingsContainer = document.getElementById('settings-container');

    // Initialize settings panel
    settingsPanel.init(settingsContainer, {
        onScanRequested: (folderPath) => {
            scanFolder(folderPath).catch(console.error);
        },
    });

    // Initialize scan table
    scanTable.init(mainContent, {
        emptyState,
        onSettingsClick: () => settingsPanel.show(),
    });

    // Wire toolbar gear button
    if (btnSettings) {
        btnSettings.addEventListener('click', () => settingsPanel.toggle());
    }

    // Wire empty state settings link
    if (btnOpenSettings) {
        btnOpenSettings.addEventListener('click', () => settingsPanel.show());
    }

    // On startup: load settings and populate table from cache
    try {
        const settings = await getSettings();
        if (settings.source_folder) {
            await scanTable.loadFromCache();
        }
    } catch (err) {
        console.error('Startup error:', err);
    }
});
