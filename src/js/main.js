import { api } from './api.js';

// Entry point — UI initialization will be implemented in Phase 03
document.addEventListener('DOMContentLoaded', () => {
    const btnSettings = document.getElementById('btn-settings');
    const btnOpenSettings = document.getElementById('btn-open-settings');

    if (btnSettings) {
        btnSettings.addEventListener('click', () => {
            // Settings panel toggle — implemented in Phase 03
            console.log('Settings clicked');
        });
    }

    if (btnOpenSettings) {
        btnOpenSettings.addEventListener('click', () => {
            if (btnSettings) btnSettings.click();
        });
    }
});
