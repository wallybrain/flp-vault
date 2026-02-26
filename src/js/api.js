// Tauri invoke wrappers — Plan 03 implementation

const { invoke } = window.__TAURI__?.core ?? { invoke: async () => { throw new Error('Tauri not available') } };
const { listen } = window.__TAURI__?.event ?? { listen: async () => { throw new Error('Tauri not available') } };

export function scanFolder(path) {
    return invoke('scan_folder', { path });
}

export function cancelScan() {
    return invoke('cancel_scan');
}

export function getSettings() {
    return invoke('get_settings');
}

export function saveSettings(settings) {
    return invoke('save_settings', { settings });
}

export function listScannedFiles() {
    return invoke('list_scanned_files');
}

export function onScanStarted(callback) {
    return listen('scan:started', callback);
}

export function onScanProgress(callback) {
    return listen('scan:progress', callback);
}

export function onScanComplete(callback) {
    return listen('scan:complete', callback);
}

export function onScanCancelled(callback) {
    return listen('scan:cancelled', callback);
}
