---
status: complete
phase: 02-grouping
source: [02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md]
started: 2026-02-26T02:45:00Z
updated: 2026-02-26T02:50:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Review Groups button appears after scan
expected: After scanning a folder, a "Review Groups" button appears in the toolbar. Before scanning, this button is not visible.
result: skipped
reason: Need to build and test on Windows first

### 2. Group proposals display with confidence coloring
expected: Clicking "Review Groups" shows a panel of group cards. Each group shows a canonical name, confidence score, and list of member files. Cards are color-coded: red (low confidence, < 0.50), yellow/orange (medium, 0.50-0.84), green (high, >= 0.85). Groups are sorted lowest confidence first.
result: skipped
reason: Need to build and test on Windows first

### 3. Pagination works
expected: If there are more than 20 groups, pagination controls appear at the bottom. Clicking next/previous navigates between pages. Page count is displayed.
result: skipped
reason: Need to build and test on Windows first

### 4. Merge two groups
expected: Clicking "Merge" on a group shows a dropdown of other groups. Selecting one combines both groups' files under a single canonical name. The merged group disappears from the list.
result: skipped
reason: Need to build and test on Windows first

### 5. Split files from a group
expected: Clicking "Split" on a group shows checkboxes next to each file. Selecting files and confirming creates a new group with those files, removing them from the original group.
result: skipped
reason: Need to build and test on Windows first

### 6. Rename canonical name
expected: Clicking the group name makes it editable inline. Typing a new name and pressing Enter (or clicking away) updates the canonical name displayed on the card.
result: skipped
reason: Need to build and test on Windows first

### 7. Assign ungrouped file to a group
expected: Ungrouped files (single-file groups) show an "Assign" option. Clicking it shows a dropdown of existing groups. Selecting one moves the file into that group.
result: skipped
reason: Need to build and test on Windows first

### 8. Ignore and un-ignore a file
expected: Each file in a group has an "Ignore" button. Clicking it visually marks the file as ignored (dimmed/struck-through). An "Un-ignore" button appears to reverse it.
result: skipped
reason: Need to build and test on Windows first

### 9. Approve All High Confidence
expected: An "Approve All High Confidence" button confirms all groups with confidence >= 0.85 in one click. Those groups are removed from the proposal list and persisted to the database.
result: skipped
reason: Need to build and test on Windows first

### 10. Confirm All Groups
expected: A "Confirm All" button persists all remaining group proposals to the database. The review panel closes and returns to the scan view.
result: skipped
reason: Need to build and test on Windows first

### 11. Close guard warns about unsaved edits
expected: If you have made any changes to group proposals (merge/split/rename/assign/ignore) and try to close the app window, a confirmation dialog appears warning about unsaved changes.
result: skipped
reason: Need to build and test on Windows first

### 12. Confirmed groups persist after restart
expected: After confirming groups and restarting the app, clicking "Review Groups" or checking the database shows the previously confirmed groups are still there (via list_groups).
result: skipped
reason: Need to build and test on Windows first

## Summary

total: 12
passed: 0
issues: 0
pending: 0
skipped: 12

## Gaps

[none yet]
