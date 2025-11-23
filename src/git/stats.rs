// Stats module
// The main RepoStats struct is defined in git/mod.rs
// This module can contain additional statistics utilities

use crate::git::{Commit, RepoStats};
use std::collections::HashMap;

/// Calculate commit frequency over time
#[allow(dead_code)]
pub fn calculate_commit_frequency(commits: &[Commit]) -> HashMap<String, u32> {
    let mut frequency = HashMap::new();

    for commit in commits {
        let date = commit.timestamp.format("%Y-%m-%d").to_string();
        *frequency.entry(date).or_insert(0) += 1;
    }

    frequency
}

/// Find the most active day
#[allow(dead_code)]
pub fn find_most_active_day(stats: &RepoStats) -> Option<(String, u32)> {
    stats
        .commit_frequency
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(date, count)| (date.clone(), *count))
}

/// Calculate average commits per day
#[allow(dead_code)]
pub fn average_commits_per_day(stats: &RepoStats) -> f64 {
    if stats.commit_frequency.is_empty() {
        return 0.0;
    }

    stats.total_commits as f64 / stats.commit_frequency.len() as f64
}

/// Get a summary of file changes
#[allow(dead_code)]
pub fn summarize_file_changes(commits: &[Commit]) -> HashMap<String, u32> {
    let mut file_changes: HashMap<String, u32> = HashMap::new();

    for commit in commits {
        for file in &commit.files_changed {
            *file_changes.entry(file.clone()).or_insert(0) += 1;
        }
    }

    file_changes
}

/// Find the most frequently changed files
#[allow(dead_code)]
pub fn most_changed_files(commits: &[Commit], limit: usize) -> Vec<(String, u32)> {
    let file_changes = summarize_file_changes(commits);

    let mut changes: Vec<_> = file_changes.into_iter().collect();
    changes.sort_by(|a, b| b.1.cmp(&a.1));
    changes.truncate(limit);

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Author;
    use chrono::Utc;

    fn create_test_commit(files: Vec<String>, insertions: u32, deletions: u32) -> Commit {
        Commit {
            hash: "abc123".to_string(),
            short_hash: "abc123".to_string(),
            author: Author {
                name: "Test".to_string(),
                email: "test@example.com".to_string(),
            },
            timestamp: Utc::now(),
            message: "Test".to_string(),
            summary: "Test".to_string(),
            body: None,
            files_changed: files,
            insertions,
            deletions,
            pr_numbers: vec![],
        }
    }

    #[test]
    fn test_calculate_commit_frequency() {
        let commits = vec![
            create_test_commit(vec![], 10, 5),
            create_test_commit(vec![], 20, 10),
        ];

        let frequency = calculate_commit_frequency(&commits);
        assert!(!frequency.is_empty());
    }

    #[test]
    fn test_summarize_file_changes() {
        let commits = vec![
            create_test_commit(vec!["file1.rs".to_string(), "file2.rs".to_string()], 10, 5),
            create_test_commit(vec!["file1.rs".to_string()], 5, 2),
        ];

        let changes = summarize_file_changes(&commits);
        assert_eq!(changes.get("file1.rs"), Some(&2));
        assert_eq!(changes.get("file2.rs"), Some(&1));
    }

    #[test]
    fn test_most_changed_files() {
        let commits = vec![
            create_test_commit(
                vec![
                    "a.rs".to_string(),
                    "b.rs".to_string(),
                    "c.rs".to_string(),
                ],
                10,
                5,
            ),
            create_test_commit(vec!["a.rs".to_string(), "b.rs".to_string()], 5, 2),
            create_test_commit(vec!["a.rs".to_string()], 3, 1),
        ];

        let top_files = most_changed_files(&commits, 2);
        assert_eq!(top_files.len(), 2);
        assert_eq!(top_files[0].0, "a.rs");
        assert_eq!(top_files[0].1, 3); // Changed 3 times
    }

    #[test]
    fn test_average_commits_per_day() {
        let commits = vec![
            create_test_commit(vec![], 10, 5),
            create_test_commit(vec![], 20, 10),
        ];

        let stats = RepoStats::from_commits(&commits);
        let avg = average_commits_per_day(&stats);
        assert!(avg > 0.0);
    }
}
