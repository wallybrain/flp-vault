// Group review panel — merge, split, rename, assign, ignore actions
// All operations are in-memory only. No file copy/move operations occur here.

import { proposeGroups, confirmGroups, listScannedFiles } from '../api.js';

// Module state
let proposals = [];
let ignoredHashes = new Set();
let currentPage = 0;
const PAGE_SIZE = 20;
let fileDetailsMap = {};
let containerEl = null;
let isDirty = false;
let splitMode = null; // { groupId, checkedHashes: Set }

// ===== Utilities =====

function makeEl(tag, props) {
    const el = document.createElement(tag);
    if (props) {
        if (props.className) el.className = props.className;
        if (props.title) el.title = props.title;
        if (props.textContent != null) el.textContent = props.textContent;
        if (props.style) Object.assign(el.style, props.style);
        if (props.type) el.type = props.type;
        if (props.placeholder) el.placeholder = props.placeholder;
        if (props.value != null) el.value = props.value;
    }
    return el;
}

function formatDate(mtime) {
    if (!mtime) return '\u2014';
    return new Date(mtime * 1000).toLocaleDateString(undefined, {
        year: 'numeric', month: 'short', day: 'numeric',
    });
}

function confidenceLabel(confidence) {
    if (confidence < 0.65) return { label: 'LOW', cls: 'confidence-low' };
    if (confidence < 0.85) return { label: 'MEDIUM', cls: 'confidence-medium' };
    return { label: 'HIGH', cls: 'confidence-high' };
}

function pct(confidence) {
    return Math.round(confidence * 100);
}

function getFilename(path) {
    return path ? path.split(/[\\/]/).pop() : path;
}

function clearEl(el) {
    while (el.firstChild) el.removeChild(el.firstChild);
}

// ===== Exports =====

export async function init(container) {
    containerEl = container;
    isDirty = false;
    ignoredHashes = new Set();
    currentPage = 0;
    splitMode = null;

    try {
        proposals = await proposeGroups();
    } catch (err) {
        console.error('propose_groups failed:', err);
        proposals = [];
    }

    // Sort ascending by confidence (lowest/hardest first)
    proposals.sort((a, b) => a.confidence - b.confidence);

    try {
        const files = await listScannedFiles();
        fileDetailsMap = {};
        for (const f of files) {
            fileDetailsMap[f.hash] = f;
        }
    } catch (err) {
        console.error('list_scanned_files failed:', err);
        fileDetailsMap = {};
    }

    renderPage(currentPage);
}

export function show() {
    if (containerEl) containerEl.style.display = '';
}

export function hide() {
    if (containerEl) containerEl.style.display = 'none';
}

export function hasUnsavedEdits() {
    return isDirty;
}

// ===== Rendering =====

function renderPage(page) {
    if (!containerEl) return;
    clearEl(containerEl);

    const grouped = proposals.filter(p => !p.is_ungrouped);
    const ungrouped = proposals.filter(p => p.is_ungrouped);

    const totalPages = Math.max(1, Math.ceil(grouped.length / PAGE_SIZE));
    currentPage = Math.min(page, totalPages - 1);
    const start = currentPage * PAGE_SIZE;
    const end = Math.min(start + PAGE_SIZE, grouped.length);
    const pageGroups = grouped.slice(start, end);

    // Header bar
    const header = makeEl('div', { className: 'review-header' });

    const counter = makeEl('span', { className: 'review-counter' });
    counter.textContent = `Groups ${start + 1}\u2013${end} of ${grouped.length}`;
    header.appendChild(counter);

    const highConf = grouped.filter(g => g.confidence >= 0.85 && !allFilesIgnored(g));
    const btnApproveAll = makeEl('button', {
        className: 'btn-action btn-approve-all',
        type: 'button',
        textContent: `Approve ${highConf.length} High-Confidence Groups`,
    });
    btnApproveAll.addEventListener('click', () => handleApproveAllHighConf());
    header.appendChild(btnApproveAll);

    containerEl.appendChild(header);

    // Pagination row (top)
    containerEl.appendChild(buildPaginationRow(currentPage, totalPages));

    // Group cards
    pageGroups.forEach((group, idx) => {
        containerEl.appendChild(renderGroupCard(group, start + idx, grouped.length + ungrouped.length));
    });

    // Pagination row (bottom)
    containerEl.appendChild(buildPaginationRow(currentPage, totalPages));

    // Ungrouped section
    if (ungrouped.length > 0) {
        containerEl.appendChild(renderUngroupedSection(ungrouped, grouped));
    }

    // Ignored files collapsible
    const allIgnored = [...ignoredHashes];
    if (allIgnored.length > 0) {
        containerEl.appendChild(renderIgnoredSection(allIgnored));
    }

    // Footer
    containerEl.appendChild(buildFooter());
}

function buildPaginationRow(page, totalPages) {
    const row = makeEl('div', { className: 'review-pagination' });

    const btnPrev = makeEl('button', { type: 'button', textContent: '\u2190 Previous' });
    btnPrev.disabled = page === 0;
    btnPrev.addEventListener('click', () => renderPage(currentPage - 1));
    row.appendChild(btnPrev);

    const pageLabel = makeEl('span');
    pageLabel.textContent = `Page ${page + 1} of ${totalPages}`;
    row.appendChild(pageLabel);

    const btnNext = makeEl('button', { type: 'button', textContent: 'Next \u2192' });
    btnNext.disabled = page >= totalPages - 1;
    btnNext.addEventListener('click', () => renderPage(currentPage + 1));
    row.appendChild(btnNext);

    return row;
}

function allFilesIgnored(group) {
    return group.file_hashes.length > 0 && group.file_hashes.every(h => ignoredHashes.has(h));
}

function renderGroupCard(group, index, totalCount) {
    const isAllIgnored = allFilesIgnored(group);
    const card = makeEl('div', { className: 'group-card' + (isAllIgnored ? ' group-card-ignored' : '') });

    // Header row
    const cardHeader = makeEl('div', { className: 'group-card-header' });

    const { label, cls } = confidenceLabel(group.confidence);
    const confEl = makeEl('span', { className: cls });
    confEl.textContent = `Group ${index + 1} of ${totalCount} \u2014 Confidence: ${pct(group.confidence)}% (${label})`;
    cardHeader.appendChild(confEl);

    // Canonical name (editable)
    const nameInput = makeEl('input', {
        type: 'text',
        className: 'canonical-input',
        value: group.canonical_name,
        placeholder: 'Group name',
    });
    nameInput.addEventListener('blur', () => {
        group.canonical_name = nameInput.value.trim() || group.canonical_name;
        isDirty = true;
    });
    nameInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            nameInput.blur();
        }
    });
    cardHeader.appendChild(nameInput);

    card.appendChild(cardHeader);

    // File table
    const table = makeEl('table', { className: 'file-table' });
    const thead = makeEl('thead');
    const headerTr = makeEl('tr');
    ['File', 'BPM', 'Date', 'Actions'].forEach(col => {
        headerTr.appendChild(makeEl('th', { textContent: col }));
    });
    thead.appendChild(headerTr);
    table.appendChild(thead);

    const tbody = makeEl('tbody');
    const inSplitMode = splitMode && splitMode.groupId === group.id;

    for (const hash of group.file_hashes) {
        const detail = fileDetailsMap[hash] || {};
        const isIgnored = ignoredHashes.has(hash);
        const tr = makeEl('tr', { className: isIgnored ? 'ignored-file' : '' });

        // Checkbox for split mode
        if (inSplitMode) {
            const tdCheck = makeEl('td');
            const cb = makeEl('input', { type: 'checkbox' });
            cb.checked = splitMode.checkedHashes.has(hash);
            cb.addEventListener('change', () => {
                if (cb.checked) splitMode.checkedHashes.add(hash);
                else splitMode.checkedHashes.delete(hash);
            });
            tdCheck.appendChild(cb);
            tr.appendChild(tdCheck);
        }

        tr.appendChild(makeEl('td', { title: detail.path || hash, textContent: getFilename(detail.path || hash) }));
        tr.appendChild(makeEl('td', {
            textContent: detail.bpm != null ? Number(detail.bpm).toFixed(0) : '\u2014',
        }));
        tr.appendChild(makeEl('td', { textContent: formatDate(detail.mtime) }));

        const tdAction = makeEl('td');
        if (isIgnored) {
            const btnUnignore = makeEl('button', { type: 'button', textContent: 'Un-ignore', className: 'btn-action btn-sm' });
            btnUnignore.addEventListener('click', () => {
                ignoredHashes.delete(hash);
                isDirty = true;
                renderPage(currentPage);
            });
            tdAction.appendChild(btnUnignore);
        } else {
            const btnIgnore = makeEl('button', { type: 'button', textContent: 'Ignore', className: 'btn-action btn-sm btn-ignore' });
            btnIgnore.addEventListener('click', () => {
                ignoredHashes.add(hash);
                isDirty = true;
                renderPage(currentPage);
            });
            tdAction.appendChild(btnIgnore);
        }
        tr.appendChild(tdAction);
        tbody.appendChild(tr);
    }

    table.appendChild(tbody);
    card.appendChild(table);

    // Action buttons row
    const actionsRow = makeEl('div', { className: 'group-actions' });

    if (inSplitMode) {
        const btnConfirmSplit = makeEl('button', { type: 'button', textContent: 'Confirm Split', className: 'btn-action btn-split' });
        btnConfirmSplit.addEventListener('click', () => handleConfirmSplit(group));
        actionsRow.appendChild(btnConfirmSplit);

        const btnCancelSplit = makeEl('button', { type: 'button', textContent: 'Cancel Split', className: 'btn-action' });
        btnCancelSplit.addEventListener('click', () => {
            splitMode = null;
            renderPage(currentPage);
        });
        actionsRow.appendChild(btnCancelSplit);
    } else {
        const btnSplit = makeEl('button', { type: 'button', textContent: 'Split', className: 'btn-action btn-split' });
        btnSplit.disabled = group.file_hashes.length < 2;
        btnSplit.addEventListener('click', () => {
            splitMode = { groupId: group.id, checkedHashes: new Set() };
            renderPage(currentPage);
        });
        actionsRow.appendChild(btnSplit);

        const btnMerge = makeEl('button', { type: 'button', textContent: 'Merge with\u2026', className: 'btn-action btn-merge' });
        btnMerge.addEventListener('click', () => showMergeDropdown(group, btnMerge));
        actionsRow.appendChild(btnMerge);
    }

    card.appendChild(actionsRow);
    return card;
}

function renderUngroupedSection(ungrouped, grouped) {
    const section = makeEl('div', { className: 'ungrouped-section' });
    const title = makeEl('h3', { textContent: `Ungrouped Files (${ungrouped.length})` });
    section.appendChild(title);

    for (const group of ungrouped) {
        for (const hash of group.file_hashes) {
            const detail = fileDetailsMap[hash] || {};
            const row = makeEl('div', { className: 'ungrouped-row' });

            const nameSpan = makeEl('span', {
                title: detail.path || hash,
                textContent: getFilename(detail.path || hash),
            });
            row.appendChild(nameSpan);

            const assignLabel = makeEl('span', { textContent: ' \u2192 ' });
            row.appendChild(assignLabel);

            const select = makeEl('select', { className: 'assign-select' });
            const defaultOpt = makeEl('option', { textContent: '-- Assign to group --', value: '' });
            select.appendChild(defaultOpt);

            for (const g of grouped) {
                const opt = makeEl('option', { textContent: g.canonical_name, value: g.id });
                select.appendChild(opt);
            }

            select.addEventListener('change', () => {
                const targetId = select.value;
                if (!targetId) return;
                handleAssignUngrouped(hash, group.id, targetId);
            });

            row.appendChild(select);
            section.appendChild(row);
        }
    }

    return section;
}

function renderIgnoredSection(hashes) {
    const section = makeEl('div', { className: 'ignored-section' });

    const toggle = makeEl('button', {
        type: 'button',
        className: 'btn-action',
        textContent: `Ignored Files (${hashes.length}) \u25bc`,
    });

    const content = makeEl('div', { className: 'ignored-list', style: { display: 'none' } });

    for (const hash of hashes) {
        const detail = fileDetailsMap[hash] || {};
        const row = makeEl('div', { className: 'ignored-row ignored-file' });

        row.appendChild(makeEl('span', {
            title: detail.path || hash,
            textContent: getFilename(detail.path || hash),
        }));

        const btnUnignore = makeEl('button', { type: 'button', textContent: 'Un-ignore', className: 'btn-action btn-sm' });
        btnUnignore.addEventListener('click', () => {
            ignoredHashes.delete(hash);
            isDirty = true;
            renderPage(currentPage);
        });
        row.appendChild(btnUnignore);
        content.appendChild(row);
    }

    toggle.addEventListener('click', () => {
        const open = content.style.display !== 'none';
        content.style.display = open ? 'none' : '';
        toggle.textContent = `Ignored Files (${hashes.length}) ${open ? '\u25bc' : '\u25b2'}`;
    });

    section.appendChild(toggle);
    section.appendChild(content);
    return section;
}

function buildFooter() {
    const footer = makeEl('div', { className: 'review-footer' });

    const btnConfirmAll = makeEl('button', {
        type: 'button',
        className: 'btn-action btn-confirm-all',
        textContent: 'Confirm All Groups',
    });
    btnConfirmAll.addEventListener('click', () => handleConfirmAll());
    footer.appendChild(btnConfirmAll);

    const btnCancel = makeEl('button', {
        type: 'button',
        className: 'btn-action',
        textContent: 'Cancel Review',
    });
    btnCancel.addEventListener('click', () => {
        if (containerEl) {
            containerEl.dispatchEvent(new CustomEvent('review:cancel'));
        }
    });
    footer.appendChild(btnCancel);

    return footer;
}

// ===== Action Handlers =====

function showMergeDropdown(sourceGroup, anchorBtn) {
    // Remove any existing merge dropdown
    const existing = document.querySelector('.merge-dropdown');
    if (existing) existing.remove();

    const others = proposals.filter(p => !p.is_ungrouped && p.id !== sourceGroup.id);
    if (others.length === 0) return;

    const dropdown = makeEl('div', { className: 'merge-dropdown' });

    const label = makeEl('div', { className: 'merge-dropdown-label', textContent: 'Merge into:' });
    dropdown.appendChild(label);

    if (others.length > 10) {
        const searchInput = makeEl('input', {
            type: 'text',
            placeholder: 'Search groups\u2026',
            className: 'merge-search',
        });
        dropdown.appendChild(searchInput);

        const list = makeEl('div', { className: 'merge-list' });
        const renderList = (filter) => {
            clearEl(list);
            const filtered = filter
                ? others.filter(g => g.canonical_name.toLowerCase().includes(filter.toLowerCase()))
                : others;
            for (const g of filtered) {
                list.appendChild(buildMergeOption(sourceGroup, g, dropdown));
            }
        };
        searchInput.addEventListener('input', () => renderList(searchInput.value));
        renderList('');
        dropdown.appendChild(list);
    } else {
        for (const g of others) {
            dropdown.appendChild(buildMergeOption(sourceGroup, g, dropdown));
        }
    }

    const btnClose = makeEl('button', { type: 'button', textContent: 'Cancel', className: 'btn-action btn-sm' });
    btnClose.addEventListener('click', () => dropdown.remove());
    dropdown.appendChild(btnClose);

    anchorBtn.parentNode.insertBefore(dropdown, anchorBtn.nextSibling);
}

function buildMergeOption(sourceGroup, targetGroup, dropdown) {
    const opt = makeEl('button', {
        type: 'button',
        className: 'merge-option',
        textContent: targetGroup.canonical_name,
    });
    opt.addEventListener('click', () => {
        handleMerge(sourceGroup.id, targetGroup.id);
        dropdown.remove();
    });
    return opt;
}

function handleMerge(sourceId, targetId) {
    const source = proposals.find(p => p.id === sourceId);
    const target = proposals.find(p => p.id === targetId);
    if (!source || !target) return;

    // Combine file_hashes, deduplicate
    const combined = [...new Set([...target.file_hashes, ...source.file_hashes])];
    target.file_hashes = combined;
    // Confidence is minimum of the two (conservative)
    target.confidence = Math.min(target.confidence, source.confidence);

    // Remove source
    proposals = proposals.filter(p => p.id !== sourceId);
    isDirty = true;

    renderPage(currentPage);
}

function handleConfirmSplit(group) {
    if (!splitMode || splitMode.groupId !== group.id) return;
    const toSplit = [...splitMode.checkedHashes];
    if (toSplit.length === 0 || toSplit.length >= group.file_hashes.length) {
        splitMode = null;
        renderPage(currentPage);
        return;
    }

    // Remove split files from original group
    group.file_hashes = group.file_hashes.filter(h => !splitMode.checkedHashes.has(h));

    // Create new group with split files
    const newGroup = {
        id: crypto.randomUUID(),
        canonical_name: group.canonical_name + ' (split)',
        confidence: 0.0,
        file_hashes: toSplit,
        is_ungrouped: false,
    };

    proposals.push(newGroup);
    // Re-sort to keep ascending confidence order
    proposals.sort((a, b) => a.confidence - b.confidence);

    splitMode = null;
    isDirty = true;
    renderPage(currentPage);
}

function handleAssignUngrouped(hash, ungroupedGroupId, targetGroupId) {
    const target = proposals.find(p => p.id === targetGroupId);
    const ungroupedGroup = proposals.find(p => p.id === ungroupedGroupId);
    if (!target) return;

    // Add hash to target group
    if (!target.file_hashes.includes(hash)) {
        target.file_hashes.push(hash);
    }

    // Remove hash from ungrouped group
    if (ungroupedGroup) {
        ungroupedGroup.file_hashes = ungroupedGroup.file_hashes.filter(h => h !== hash);
        // If ungrouped group is now empty, remove it entirely
        if (ungroupedGroup.file_hashes.length === 0) {
            proposals = proposals.filter(p => p.id !== ungroupedGroupId);
        }
    }

    isDirty = true;
    renderPage(currentPage);
}

function handleApproveAllHighConf() {
    const highConf = proposals.filter(
        p => !p.is_ungrouped && p.confidence >= 0.85 && !allFilesIgnored(p)
    );
    if (highConf.length === 0) return;

    // Build confirmations for high-confidence groups
    const confirmations = highConf.map(g => buildGroupConfirmation(g));

    confirmGroups(confirmations)
        .then(() => {
            // Remove confirmed groups from proposals
            const confirmedIds = new Set(highConf.map(g => g.id));
            proposals = proposals.filter(p => !confirmedIds.has(p.id));
            isDirty = proposals.length > 0;
            renderPage(0);
        })
        .catch(err => {
            console.error('confirm_groups failed:', err);
        });
}

async function handleConfirmAll() {
    const confirmations = proposals
        .filter(p => !p.is_ungrouped && !allFilesIgnored(p))
        .map(g => buildGroupConfirmation(g));

    // Include ungrouped files that have non-ignored hashes
    const ungroupedConfirmations = proposals
        .filter(p => p.is_ungrouped && !allFilesIgnored(p))
        .map(g => buildGroupConfirmation(g));

    const allConfirmations = [...confirmations, ...ungroupedConfirmations];

    try {
        await confirmGroups(allConfirmations);
        isDirty = false;
        if (containerEl) {
            containerEl.dispatchEvent(new CustomEvent('review:confirmed'));
        }
    } catch (err) {
        console.error('confirm_groups failed:', err);
        const errMsg = makeEl('div', { className: 'review-error', textContent: `Error: ${err}` });
        containerEl.prepend(errMsg);
        setTimeout(() => errMsg.remove(), 5000);
    }
}

function buildGroupConfirmation(group) {
    const groupIgnored = group.file_hashes.filter(h => ignoredHashes.has(h));
    const activeHashes = group.file_hashes.filter(h => !ignoredHashes.has(h));
    return {
        canonical_name: group.canonical_name,
        file_hashes: activeHashes,
        ignored_hashes: groupIgnored,
    };
}
