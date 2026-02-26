// Scan results table with live streaming from scan events

import { onScanStarted, onScanProgress, onScanComplete, onScanCancelled, cancelScan, listScannedFiles } from '../api.js';

const COLUMNS = [
    { key: 'name',     label: 'Name',     sortFn: (a, b) => a.name.localeCompare(b.name) },
    { key: 'bpm',      label: 'BPM',      sortFn: (a, b) => (a.bpmRaw ?? -1) - (b.bpmRaw ?? -1) },
    { key: 'channels', label: 'Channels', sortFn: (a, b) => (a.channel_count ?? 0) - (b.channel_count ?? 0) },
    { key: 'plugins',  label: 'Plugins',  sortFn: (a, b) => a.pluginsSummary.localeCompare(b.pluginsSummary) },
    { key: 'modified', label: 'Modified', sortFn: (a, b) => (a.mtime ?? 0) - (b.mtime ?? 0) },
];

let rows = [];
let sortKey = 'name';
let sortDir = 1;
let scanning = false;
let scanTotal = 0;
let scanDone = 0;

let tableEl = null;
let tbodyEl = null;
let progressEl = null;
let progressBarEl = null;
let progressTextEl = null;
let emptyStateEl = null;

function parsePlugins(plugins_json) {
    if (!plugins_json) return [];
    try { return JSON.parse(plugins_json); } catch { return []; }
}

function formatDate(mtime) {
    if (!mtime) return '\u2014';
    return new Date(mtime * 1000).toLocaleDateString(undefined, {
        year: 'numeric', month: 'short', day: 'numeric',
    });
}

function rowDataFromRecord(record) {
    const plugins = parsePlugins(record.plugins_json);
    const pluginsSummary = buildPluginSummary(plugins);
    return {
        hash: record.hash,
        path: record.path,
        name: record.path.split(/[\\/]/).pop() ?? record.path,
        bpmRaw: record.bpm,
        bpm: record.bpm != null ? Number(record.bpm).toFixed(0) : null,
        channel_count: record.channel_count,
        plugins,
        pluginsSummary,
        mtime: record.mtime,
        formattedDate: formatDate(record.mtime),
        warnings: [],
    };
}

function rowDataFromProgress(payload) {
    const plugins = parsePlugins(payload.plugins_json);
    const pluginsSummary = buildPluginSummary(plugins);
    return {
        hash: payload.hash ?? payload.path,
        path: payload.path,
        name: payload.path.split(/[\\/]/).pop() ?? payload.path,
        bpmRaw: payload.bpm,
        bpm: payload.bpm != null ? Number(payload.bpm).toFixed(0) : null,
        channel_count: payload.channel_count,
        plugins,
        pluginsSummary,
        mtime: payload.mtime,
        formattedDate: formatDate(payload.mtime),
        warnings: payload.warnings ?? [],
    };
}

function buildPluginSummary(plugins) {
    if (!plugins || plugins.length === 0) return '';
    const visible = plugins.slice(0, 3);
    const extra = plugins.length - visible.length;
    return visible.join(', ') + (extra > 0 ? ` (+${extra} more)` : '');
}

function makeEl(tag, props) {
    const el = document.createElement(tag);
    if (props) {
        if (props.className) el.className = props.className;
        if (props.title) el.title = props.title;
        if (props.textContent != null) el.textContent = props.textContent;
        if (props.style) Object.assign(el.style, props.style);
        if (props.type) el.type = props.type;
    }
    return el;
}

function renderRow(row) {
    const tr = makeEl('tr');
    if (row.warnings && row.warnings.length > 0) {
        tr.classList.add('has-warnings');
        tr.title = row.warnings.join('\n');
    }

    // Name column
    const tdName = makeEl('td', { className: 'col-name', title: row.path });
    if (row.warnings && row.warnings.length > 0) {
        const warnSpan = makeEl('span', {
            className: 'warning-icon',
            title: row.warnings.join('\n'),
            textContent: '\u26a0\ufe0f',
        });
        tdName.appendChild(warnSpan);
        tdName.appendChild(document.createTextNode(' '));
    }
    tdName.appendChild(document.createTextNode(row.name));
    tr.appendChild(tdName);

    // BPM column
    const tdBpm = makeEl('td', { className: 'col-bpm' });
    if (row.bpm != null) {
        tdBpm.textContent = row.bpm;
    } else {
        const q = makeEl('span', {
            className: 'bpm-unknown',
            title: 'BPM not found in this file',
            textContent: '?',
        });
        tdBpm.appendChild(q);
    }
    tr.appendChild(tdBpm);

    // Channels column
    const tdChan = makeEl('td', {
        className: 'col-channels',
        textContent: row.channel_count != null ? String(row.channel_count) : '\u2014',
    });
    tr.appendChild(tdChan);

    // Plugins column
    const tdPlugins = makeEl('td', {
        className: 'col-plugins',
        title: row.plugins.join(', '),
        textContent: row.pluginsSummary || '\u2014',
    });
    tr.appendChild(tdPlugins);

    // Modified column
    const tdDate = makeEl('td', {
        className: 'col-modified',
        textContent: row.formattedDate,
    });
    tr.appendChild(tdDate);

    return tr;
}

function sortedRows() {
    const col = COLUMNS.find(c => c.key === sortKey);
    if (!col) return rows;
    return [...rows].sort((a, b) => col.sortFn(a, b) * sortDir);
}

function renderTable() {
    if (!tbodyEl) return;
    while (tbodyEl.firstChild) tbodyEl.removeChild(tbodyEl.firstChild);
    const sorted = sortedRows();
    for (const row of sorted) {
        tbodyEl.appendChild(renderRow(row));
    }
    updateEmptyState();
}

function updateEmptyState() {
    if (!emptyStateEl) return;
    emptyStateEl.style.display = rows.length === 0 && !scanning ? 'flex' : 'none';
    if (tableEl) tableEl.style.display = rows.length > 0 ? '' : 'none';
}

function updateProgressBar() {
    if (!progressEl) return;
    if (scanning) {
        progressEl.style.display = 'flex';
        progressTextEl.textContent = `Scanning\u2026 ${scanDone}/${scanTotal} files`;
        const pct = scanTotal > 0 ? (scanDone / scanTotal) * 100 : 0;
        progressBarEl.style.width = `${pct}%`;
    } else {
        progressEl.style.display = 'none';
    }
}

function setSortColumn(key) {
    if (sortKey === key) {
        sortDir = -sortDir;
    } else {
        sortKey = key;
        sortDir = 1;
    }
    updateHeaderArrows();
    renderTable();
}

function updateHeaderArrows() {
    if (!tableEl) return;
    tableEl.querySelectorAll('thead th').forEach((th) => {
        const key = th.dataset.key;
        th.classList.toggle('sorted-asc', key === sortKey && sortDir === 1);
        th.classList.toggle('sorted-desc', key === sortKey && sortDir === -1);
    });
}

function buildProgressBar(container) {
    const wrapper = makeEl('div', { className: 'progress-container', style: { display: 'none' } });

    const track = makeEl('div', { className: 'progress-track' });
    const fill = makeEl('div', { className: 'progress-fill' });
    track.appendChild(fill);
    wrapper.appendChild(track);

    const text = makeEl('span', { className: 'progress-text' });
    wrapper.appendChild(text);

    const cancelBtn = makeEl('button', { className: 'btn-cancel', textContent: 'Cancel', type: 'button' });
    cancelBtn.addEventListener('click', () => cancelScan().catch(console.error));
    wrapper.appendChild(cancelBtn);

    container.appendChild(wrapper);

    return { wrapper, fill, text };
}

function buildTable(container) {
    const table = makeEl('table', { className: 'scan-table', style: { display: 'none' } });

    const thead = makeEl('thead');
    const headerRow = makeEl('tr');
    COLUMNS.forEach(col => {
        const th = makeEl('th', { className: `col-${col.key}`, textContent: col.label });
        th.dataset.key = col.key;
        th.style.cursor = 'pointer';
        th.addEventListener('click', () => setSortColumn(col.key));
        headerRow.appendChild(th);
    });
    thead.appendChild(headerRow);
    table.appendChild(thead);

    const tbody = makeEl('tbody');
    table.appendChild(tbody);
    container.appendChild(table);

    return { table, tbody };
}

export function init(container, { emptyState }) {
    emptyStateEl = emptyState;

    const { wrapper, fill, text } = buildProgressBar(container);
    progressEl = wrapper;
    progressBarEl = fill;
    progressTextEl = text;

    const { table, tbody } = buildTable(container);
    tableEl = table;
    tbodyEl = tbody;

    updateHeaderArrows();
    updateEmptyState();

    // Scan event listeners
    onScanStarted(({ payload }) => {
        scanning = true;
        scanTotal = payload.total;
        scanDone = 0;
        rows = [];
        renderTable();
        updateProgressBar();
    }).catch(console.error);

    onScanProgress(({ payload }) => {
        scanDone = payload.done;
        scanTotal = payload.total;
        const row = rowDataFromProgress(payload);
        const existing = rows.findIndex(r => r.path === row.path);
        if (existing >= 0) {
            rows[existing] = row;
        } else {
            rows.push(row);
        }
        renderTable();
        updateProgressBar();
    }).catch(console.error);

    onScanComplete(() => {
        scanning = false;
        updateProgressBar();
        updateEmptyState();
    }).catch(console.error);

    onScanCancelled(() => {
        scanning = false;
        updateProgressBar();
        updateEmptyState();
    }).catch(console.error);
}

export async function loadFromCache() {
    try {
        const records = await listScannedFiles();
        rows = records.map(rowDataFromRecord);
        renderTable();
    } catch (err) {
        console.error('Failed to load cached files:', err);
    }
}
