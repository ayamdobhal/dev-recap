pub mod github;
pub mod parser;
pub mod scanner;
pub mod stats;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Information about a git repository
#[derive(Debug, Clone)]
pub struct Repository {
    /// Path to the repository
    pub path: PathBuf,
    /// Repository name (derived from directory name)
    pub name: String,
    /// Remote URL (if available)
    pub remote_url: Option<String>,
    /// GitHub repository info (if applicable)
    pub github_info: Option<GitHubRepo>,
    /// Filtered commits
    pub commits: Vec<Commit>,
    /// Repository statistics
    pub stats: RepoStats,
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    /// Repository owner/organization
    pub owner: String,
    /// Repository name
    pub repo: String,
}

impl GitHubRepo {
    /// Create a GitHub PR URL
    pub fn pr_url(&self, pr_number: u32) -> String {
        format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pr_number)
    }

    /// Create a GitHub commit URL
    pub fn commit_url(&self, hash: &str) -> String {
        format!("https://github.com/{}/{}/commit/{}", self.owner, self.repo, hash)
    }
}

/// Git commit information
#[derive(Debug, Clone)]
pub struct Commit {
    /// Full commit hash
    pub hash: String,
    /// Short commit hash (7 characters)
    pub short_hash: String,
    /// Commit author
    pub author: Author,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Full commit message
    pub message: String,
    /// First line of commit message
    pub summary: String,
    /// Rest of commit message (if any)
    pub body: Option<String>,
    /// List of files changed
    pub files_changed: Vec<String>,
    /// Number of insertions
    pub insertions: u32,
    /// Number of deletions
    pub deletions: u32,
    /// PR numbers mentioned in commit message
    pub pr_numbers: Vec<u32>,
}

impl Commit {
    /// Create a short representation of the commit
    pub fn short_desc(&self) -> String {
        format!("{} - {}", self.short_hash, self.summary)
    }
}

/// Commit author information
#[derive(Debug, Clone)]
pub struct Author {
    /// Author name
    pub name: String,
    /// Author email
    pub email: String,
}

/// Repository statistics
#[derive(Debug, Clone, Default)]
pub struct RepoStats {
    /// Total number of commits
    pub total_commits: u32,
    /// Total files changed across all commits
    pub total_files_changed: u32,
    /// Total insertions
    pub total_insertions: u32,
    /// Total deletions
    pub total_deletions: u32,
    /// Number of unique PRs mentioned
    pub pr_count: u32,
    /// Commits per day (date string -> count)
    pub commit_frequency: std::collections::HashMap<String, u32>,
}

impl RepoStats {
    /// Create statistics from a list of commits
    pub fn from_commits(commits: &[Commit]) -> Self {
        let mut stats = Self::default();
        let mut pr_set = std::collections::HashSet::new();

        for commit in commits {
            stats.total_commits += 1;
            stats.total_files_changed += commit.files_changed.len() as u32;
            stats.total_insertions += commit.insertions;
            stats.total_deletions += commit.deletions;

            // Track PRs
            for pr in &commit.pr_numbers {
                pr_set.insert(*pr);
            }

            // Track commit frequency by date
            let date = commit.timestamp.format("%Y-%m-%d").to_string();
            *stats.commit_frequency.entry(date).or_insert(0) += 1;
        }

        stats.pr_count = pr_set.len() as u32;
        stats
    }

    /// Get net lines changed (insertions - deletions)
    pub fn net_lines_changed(&self) -> i64 {
        self.total_insertions as i64 - self.total_deletions as i64
    }
}

/// Timespan for filtering commits
#[derive(Debug, Clone)]
pub struct Timespan {
    /// Start date (inclusive)
    pub start: DateTime<Utc>,
    /// End date (inclusive)
    pub end: DateTime<Utc>,
}

impl Timespan {
    /// Create a timespan from days back from now
    pub fn days_back(days: u32) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(days as i64);
        Self { start, end }
    }

    /// Create a timespan from specific dates
    pub fn from_dates(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Check if a date is within this timespan
    pub fn contains(&self, date: &DateTime<Utc>) -> bool {
        date >= &self.start && date <= &self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_repo_urls() {
        let repo = GitHubRepo {
            owner: "rust-lang".to_string(),
            repo: "rust".to_string(),
        };
        assert_eq!(repo.pr_url(123), "https://github.com/rust-lang/rust/pull/123");
        assert_eq!(repo.commit_url("abc123"), "https://github.com/rust-lang/rust/commit/abc123");
    }

    #[test]
    fn test_timespan_days_back() {
        let now = Utc::now();
        let timespan = Timespan::days_back(7);

        // Test that recent date is within timespan
        let recent_date = now - chrono::Duration::days(3);
        assert!(timespan.contains(&recent_date));

        // Test that old date is outside timespan
        let old_date = now - chrono::Duration::days(10);
        assert!(!timespan.contains(&old_date));
    }

    #[test]
    fn test_repo_stats() {
        let commits = vec![
            Commit {
                hash: "abc123".to_string(),
                short_hash: "abc123".to_string(),
                author: Author {
                    name: "Test".to_string(),
                    email: "test@example.com".to_string(),
                },
                timestamp: Utc::now(),
                message: "Test commit #123".to_string(),
                summary: "Test commit".to_string(),
                body: None,
                files_changed: vec!["file1.rs".to_string(), "file2.rs".to_string()],
                insertions: 10,
                deletions: 5,
                pr_numbers: vec![123],
            },
        ];

        let stats = RepoStats::from_commits(&commits);
        assert_eq!(stats.total_commits, 1);
        assert_eq!(stats.total_files_changed, 2);
        assert_eq!(stats.total_insertions, 10);
        assert_eq!(stats.total_deletions, 5);
        assert_eq!(stats.pr_count, 1);
        assert_eq!(stats.net_lines_changed(), 5);
    }
}
