// Tauri invoke wrappers — implemented in Phase 03 alongside Rust commands

const { invoke } = window.__TAURI__?.core ?? { invoke: async () => { throw new Error('Tauri not available') } };

export const api = {
    // Scan commands (Phase 03)
    startScan: () => invoke('start_scan'),
    cancelScan: () => invoke('cancel_scan'),
    getScanStatus: () => invoke('get_scan_status'),

    // Settings commands (Phase 03)
    getSettings: () => invoke('get_settings'),
    setSettings: (settings) => invoke('set_settings', { settings }),

    // File listing commands (Phase 03)
    listFiles: () => invoke('list_files'),
};
