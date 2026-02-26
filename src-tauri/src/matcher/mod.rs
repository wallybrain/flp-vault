pub mod normalize;
pub mod scorer;
pub mod signals;
pub mod union_find;

use crate::store::files::FileRecord;
use normalize::normalize_filename;
use scorer::compute_confidence;
use union_find::UnionFind;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProposedGroup {
    pub id: String,
    pub canonical_name: String,
    pub confidence: f32,
    pub file_hashes: Vec<String>,
    pub is_ungrouped: bool,
}

pub fn propose_groups(files: &[FileRecord], threshold: f32) -> Vec<ProposedGroup> {
    if files.is_empty() {
        return vec![];
    }

    let n = files.len();
    let normalized: Vec<String> = files.iter().map(|f| normalize_filename(&f.path)).collect();

    let mut uf = UnionFind::new(n);
    // Track minimum edge confidence per connected component (keyed by pair)
    let mut edge_confidences: Vec<Vec<Option<f32>>> = vec![vec![None; n]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            let conf = compute_confidence(
                &normalized[i],
                &normalized[j],
                files[i].bpm,
                files[j].bpm,
                files[i].mtime,
                files[j].mtime,
            );
            if conf >= threshold {
                uf.union(i, j);
                edge_confidences[i][j] = Some(conf);
                edge_confidences[j][i] = Some(conf);
            }
        }
    }

    let component_map = uf.groups();
    let mut groups: Vec<ProposedGroup> = Vec::new();

    for (_root, members) in component_map {
        let file_hashes: Vec<String> = members.iter().map(|&i| files[i].hash.clone()).collect();
        let is_ungrouped = members.len() == 1;

        // Compute group confidence = minimum edge confidence in the group
        // For ungrouped files, confidence = 0.0
        let confidence = if is_ungrouped {
            0.0
        } else {
            let mut min_conf = f32::MAX;
            for &i in &members {
                for &j in &members {
                    if i < j {
                        if let Some(c) = edge_confidences[i][j] {
                            if c < min_conf {
                                min_conf = c;
                            }
                        }
                    }
                }
            }
            if min_conf == f32::MAX { threshold } else { min_conf }
        };

        // Canonical name: most common normalized name, tiebreak by oldest mtime
        let canonical_name = pick_canonical_name(&members, &normalized, files);

        groups.push(ProposedGroup {
            id: Uuid::new_v4().to_string(),
            canonical_name,
            confidence,
            file_hashes,
            is_ungrouped,
        });
    }

    // Sort by confidence ascending (lowest first for review UI)
    groups.sort_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal));

    groups
}

fn pick_canonical_name(members: &[usize], normalized: &[String], files: &[FileRecord]) -> String {
    use std::collections::HashMap;

    let mut name_counts: HashMap<&str, (usize, i64)> = HashMap::new();
    for &i in members {
        let name = normalized[i].as_str();
        let entry = name_counts.entry(name).or_insert((0, i64::MAX));
        entry.0 += 1;
        // Track oldest mtime for this name
        if files[i].mtime < entry.1 {
            entry.1 = files[i].mtime;
        }
    }

    // Most common name; tiebreak by oldest mtime (smallest value)
    name_counts
        .into_iter()
        .max_by(|a, b| {
            let count_cmp = a.1.0.cmp(&b.1.0);
            if count_cmp == std::cmp::Ordering::Equal {
                // Tiebreak: older mtime wins (smaller = older)
                b.1.1.cmp(&a.1.1)
            } else {
                count_cmp
            }
        })
        .map(|(name, _)| name.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::files::FileRecord;

    fn make_record(hash: &str, path: &str, bpm: Option<f64>, mtime: i64) -> FileRecord {
        FileRecord {
            hash: hash.to_string(),
            path: path.to_string(),
            file_size: 1000,
            mtime,
            bpm,
            channel_count: Some(8),
            plugins_json: None,
            fl_version: None,
        }
    }

    #[test]
    fn test_groups_similar_filenames() {
        let files = vec![
            make_record("a", "Acid Bass Line.flp", Some(128.0), 1700000000),
            make_record("b", "Acid Bass Line 2.flp", Some(128.0), 1700086400),
            make_record("c", "Funky Groove.flp", Some(90.0), 1700000000),
        ];
        let groups = propose_groups(&files, 0.65);
        // Should produce 2 groups: one with files a+b, one ungrouped c
        assert_eq!(groups.len(), 2);
        let grouped: Vec<_> = groups.iter().filter(|g| !g.is_ungrouped).collect();
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].file_hashes.len(), 2);
    }

    #[test]
    fn test_ungrouped_files_marked() {
        let files = vec![
            make_record("a", "Completely Unique Name.flp", None, 1700000000),
            make_record("b", "Another Different Song.flp", None, 1700000000),
        ];
        let groups = propose_groups(&files, 0.65);
        assert!(groups.iter().all(|g| g.is_ungrouped));
    }

    #[test]
    fn test_sorted_by_confidence_ascending() {
        let files = vec![
            make_record("a", "Song A.flp", Some(128.0), 1700000000),
            make_record("b", "Song A 2.flp", Some(128.0), 1700086400),
            make_record("c", "Beat X.flp", Some(90.0), 1700000000),
            make_record("d", "Beat X 2.flp", Some(90.0), 1700000000),
        ];
        let groups = propose_groups(&files, 0.65);
        for i in 1..groups.len() {
            assert!(groups[i].confidence >= groups[i - 1].confidence);
        }
    }

    #[test]
    fn test_empty_input() {
        let groups = propose_groups(&[], 0.65);
        assert!(groups.is_empty());
    }
}
