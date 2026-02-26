// Settings slide-out panel with native folder pickers

import { getSettings, saveSettings, scanFolder } from '../api.js';

let panelEl = null;
let overlayEl = null;
let currentSettings = { source_folder: '', organized_folder: '', originals_folder: '' };
let previousSourceFolder = '';
let onRescan = null;

function makeEl(tag, props) {
    const el = document.createElement(tag);
    if (props) {
        if (props.className) el.className = props.className;
        if (props.textContent != null) el.textContent = props.textContent;
        if (props.title) el.title = props.title;
        if (props.type) el.type = props.type;
        if (props.placeholder) el.placeholder = props.placeholder;
        if (props.readOnly != null) el.readOnly = props.readOnly;
        if (props.value != null) el.value = props.value;
    }
    return el;
}

function buildFolderRow(label, key, placeholder) {
    const row = makeEl('div', { className: 'settings-row' });

    const lbl = makeEl('label', { className: 'settings-label', textContent: label });
    row.appendChild(lbl);

    const inputWrapper = makeEl('div', { className: 'settings-input-wrapper' });

    const input = makeEl('input', {
        className: 'settings-path-input',
        type: 'text',
        readOnly: true,
        placeholder,
        value: currentSettings[key] ?? '',
    });
    input.dataset.key = key;
    inputWrapper.appendChild(input);

    const browseBtn = makeEl('button', {
        className: 'btn-browse',
        textContent: 'Browse\u2026',
        type: 'button',
        title: `Select ${label}`,
    });
    browseBtn.addEventListener('click', async () => {
        try {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({
                directory: true,
                title: `Select ${label}`,
                defaultPath: input.value || undefined,
            });
            if (selected) {
                input.value = selected;
                currentSettings[key] = selected;
            }
        } catch (err) {
            console.error('Folder picker error:', err);
        }
    });
    inputWrapper.appendChild(browseBtn);

    row.appendChild(inputWrapper);
    return { row, input };
}

function showWarnings(warnings) {
    let warningsEl = panelEl.querySelector('.settings-warnings');
    if (!warningsEl) {
        warningsEl = makeEl('div', { className: 'settings-warnings' });
        const saveBtn = panelEl.querySelector('.btn-save-settings');
        if (saveBtn && saveBtn.parentNode) {
            saveBtn.parentNode.insertBefore(warningsEl, saveBtn);
        }
    }
    while (warningsEl.firstChild) warningsEl.removeChild(warningsEl.firstChild);
    if (warnings.length === 0) {
        warningsEl.style.display = 'none';
        return;
    }
    warningsEl.style.display = '';
    warnings.forEach(w => {
        const li = makeEl('div', { className: 'settings-warning-item', textContent: '\u26a0\ufe0f ' + w });
        warningsEl.appendChild(li);
    });
}

function buildPanel() {
    panelEl = makeEl('div', { className: 'settings-panel' });

    // Header
    const header = makeEl('div', { className: 'settings-header' });
    const title = makeEl('h2', { className: 'settings-title', textContent: 'Settings' });
    header.appendChild(title);
    const closeBtn = makeEl('button', { className: 'btn-close-panel', textContent: '\u00d7', type: 'button', title: 'Close' });
    closeBtn.addEventListener('click', hide);
    header.appendChild(closeBtn);
    panelEl.appendChild(header);

    // Body
    const body = makeEl('div', { className: 'settings-body' });

    const { row: sourceRow, input: sourceInput } = buildFolderRow(
        'Source Folder',
        'source_folder',
        'FL Studio Projects folder\u2026',
    );
    body.appendChild(sourceRow);
    sourceInput.dataset.key = 'source_folder';

    const { row: organizedRow } = buildFolderRow(
        'Organized Folder',
        'organized_folder',
        'FLP Vault output folder\u2026',
    );
    body.appendChild(organizedRow);

    const { row: originalsRow } = buildFolderRow(
        'Originals Folder',
        'originals_folder',
        'Originals backup folder\u2026',
    );
    body.appendChild(originalsRow);

    panelEl.appendChild(body);

    // Footer
    const footer = makeEl('div', { className: 'settings-footer' });
    const saveBtn = makeEl('button', { className: 'btn-save-settings btn-primary', textContent: 'Save', type: 'button' });
    saveBtn.addEventListener('click', async () => {
        const prevSource = previousSourceFolder;
        try {
            const result = await saveSettings(currentSettings);
            showWarnings(result.warnings);
            previousSourceFolder = currentSettings.source_folder;
            if (currentSettings.source_folder && currentSettings.source_folder !== prevSource) {
                hide();
                if (onRescan) onRescan(currentSettings.source_folder);
            } else {
                hide();
            }
        } catch (err) {
            console.error('Failed to save settings:', err);
        }
    });
    footer.appendChild(saveBtn);
    panelEl.appendChild(footer);

    return panelEl;
}

export function init(container, { onScanRequested }) {
    onRescan = onScanRequested;

    overlayEl = makeEl('div', { className: 'settings-overlay' });
    overlayEl.style.display = 'none';
    overlayEl.addEventListener('click', (e) => {
        if (e.target === overlayEl) hide();
    });
    container.appendChild(overlayEl);

    buildPanel();
    overlayEl.appendChild(panelEl);
}

export async function show() {
    try {
        const settings = await getSettings();
        currentSettings = { ...settings };
        previousSourceFolder = settings.source_folder;

        // Update inputs
        if (panelEl) {
            panelEl.querySelectorAll('.settings-path-input').forEach(input => {
                const key = input.dataset.key;
                if (key && currentSettings[key] != null) {
                    input.value = currentSettings[key];
                }
            });
        }

        showWarnings([]);
    } catch (err) {
        console.error('Failed to load settings:', err);
    }

    if (overlayEl) overlayEl.style.display = 'flex';
    if (panelEl) panelEl.classList.add('open');
}

export function hide() {
    if (overlayEl) overlayEl.style.display = 'none';
    if (panelEl) panelEl.classList.remove('open');
}

export function toggle() {
    if (overlayEl && overlayEl.style.display === 'none') {
        show();
    } else {
        hide();
    }
}
