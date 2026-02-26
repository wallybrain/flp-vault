import { getSettings, scanFolder, onScanComplete } from './api.js';
import * as scanTable from './panels/scan-table.js';
import * as settingsPanel from './panels/settings-panel.js';
import * as reviewPanel from './workflow/review-panel.js';

document.addEventListener('DOMContentLoaded', async () => {
    const mainContent = document.getElementById('main-content');
    const emptyState = document.getElementById('empty-state');
    const btnSettings = document.getElementById('btn-settings');
    const btnOpenSettings = document.getElementById('btn-open-settings');
    const settingsContainer = document.getElementById('settings-container');
    const btnRescan = document.getElementById('btn-rescan');
    const btnReviewGroups = document.getElementById('btn-review-groups');
    const reviewContainer = document.getElementById('review-container');

    // Track whether the review panel is currently visible
    let reviewVisible = false;

    function showScanView() {
        reviewVisible = false;
        reviewPanel.hide();
        reviewContainer.style.display = 'none';
        mainContent.style.display = '';
    }

    function showReviewView() {
        reviewVisible = true;
        mainContent.style.display = 'none';
        reviewContainer.style.display = '';
        reviewPanel.show();
    }

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

    // Show "Review Groups" button after scan completes
    onScanComplete(() => {
        if (btnReviewGroups) btnReviewGroups.style.display = '';
    }).catch(console.error);

    // Wire "Rescan" button
    if (btnRescan) {
        btnRescan.addEventListener('click', async () => {
            try {
                const settings = await getSettings();
                if (settings.source_folder) {
                    await scanFolder(settings.source_folder);
                }
            } catch (err) {
                console.error('Rescan error:', err);
            }
        });
    }

    // Wire "Review Groups" button
    if (btnReviewGroups) {
        btnReviewGroups.addEventListener('click', async () => {
            showReviewView();
            await reviewPanel.init(reviewContainer);
        });
    }

    // Wire review panel events
    reviewContainer.addEventListener('review:cancel', () => {
        showScanView();
    });

    reviewContainer.addEventListener('review:confirmed', () => {
        showScanView();
        if (btnReviewGroups) btnReviewGroups.style.display = 'none';
    });

    // Close guard — warn if review panel has unsaved edits
    if (window.__TAURI__?.window) {
        try {
            const { getCurrentWindow } = await import('@tauri-apps/api/window');
            getCurrentWindow().onCloseRequested(async (event) => {
                if (reviewVisible && reviewPanel.hasUnsavedEdits()) {
                    const confirmed = window.confirm('You have unsaved group edits. Close without saving?');
                    if (!confirmed) event.preventDefault();
                }
            });
        } catch (err) {
            // Non-Tauri environment (dev browser) — skip close guard
            console.debug('onCloseRequested not available:', err);
        }
    }

    // On startup: load settings and populate table from cache
    try {
        const settings = await getSettings();
        if (settings.source_folder) {
            if (btnRescan) btnRescan.style.display = '';
            await scanTable.loadFromCache();
        }
    } catch (err) {
        console.error('Startup error:', err);
    }
});
