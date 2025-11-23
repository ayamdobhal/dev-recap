use crate::error::Result;
use crate::git::{Author, Commit, Timespan};
use chrono::{DateTime, TimeZone, Utc};
use git2::Repository as Git2Repository;
use std::path::Path;

/// Parser for extracting commits from a git repository
pub struct Parser {
    /// Author email filter
    author_email: Option<String>,
    /// Timespan filter
    timespan: Timespan,
}

impl Parser {
    /// Create a new parser
    pub fn new(author_email: Option<String>, timespan: Timespan) -> Self {
        Self {
            author_email,
            timespan,
        }
    }

    /// Parse commits from a repository
    pub fn parse_commits(&self, repo_path: &Path) -> Result<Vec<Commit>> {
        let repo = Git2Repository::open(repo_path)?;
        let mut revwalk = repo.revwalk()?;

        // Start from HEAD
        revwalk.push_head()?;

        // Set sorting to chronological order
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::new();

        for oid in revwalk {
            let oid = oid?;
            let git_commit = repo.find_commit(oid)?;

            // Convert timestamp
            let timestamp = Self::convert_timestamp(&git_commit);

            // Filter by timespan
            if !self.timespan.contains(&timestamp) {
                continue;
            }

            // Get author info
            let author = Self::extract_author(&git_commit);

            // Filter by author email if specified
            if let Some(ref filter_email) = self.author_email {
                if !author.email.to_lowercase().contains(&filter_email.to_lowercase()) {
                    continue;
                }
            }

            // Extract commit data
            let hash = oid.to_string();
            let short_hash = format!("{:.7}", hash);
            let message = git_commit.message().unwrap_or("").to_string();
            let (summary, body) = Self::split_message(&message);

            // Get diff stats
            let (files_changed, insertions, deletions) =
                Self::get_diff_stats(&repo, &git_commit)?;

            // Detect PR numbers
            let pr_numbers = crate::git::github::extract_pr_numbers(&message);

            commits.push(Commit {
                hash,
                short_hash,
                author,
                timestamp,
                message,
                summary,
                body,
                files_changed,
                insertions,
                deletions,
                pr_numbers,
            });
        }

        Ok(commits)
    }

    /// Convert git2 Time to DateTime<Utc>
    fn convert_timestamp(commit: &git2::Commit) -> DateTime<Utc> {
        let time = commit.time();
        Utc.timestamp_opt(time.seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now)
    }

    /// Extract author information
    fn extract_author(commit: &git2::Commit) -> Author {
        let author = commit.author();
        Author {
            name: author.name().unwrap_or("Unknown").to_string(),
            email: author.email().unwrap_or("unknown@example.com").to_string(),
        }
    }

    /// Split commit message into summary and body
    fn split_message(message: &str) -> (String, Option<String>) {
        let mut lines = message.lines();
        let summary = lines.next().unwrap_or("").trim().to_string();

        // Collect remaining lines as body
        let body: Vec<&str> = lines.skip_while(|l| l.trim().is_empty()).collect();

        if body.is_empty() {
            (summary, None)
        } else {
            (summary, Some(body.join("\n")))
        }
    }

    /// Get diff statistics for a commit
    fn get_diff_stats(
        repo: &Git2Repository,
        commit: &git2::Commit,
    ) -> Result<(Vec<String>, u32, u32)> {
        let mut files_changed = Vec::new();
        let insertions;
        let deletions;

        // Get the tree for this commit
        let tree = commit.tree()?;

        // Get parent commit tree (or empty tree for first commit)
        let parent_tree = match commit.parent_count() {
            0 => None,
            _ => Some(commit.parent(0)?.tree()?),
        };

        // Create diff
        let diff = if let Some(parent_tree) = parent_tree {
            repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?
        } else {
            repo.diff_tree_to_tree(None, Some(&tree), None)?
        };

        // Get stats
        let stats = diff.stats()?;
        insertions = stats.insertions() as u32;
        deletions = stats.deletions() as u32;

        // Collect file names
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files_changed.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )?;

        Ok((files_changed, insertions, deletions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_repo_with_commits(temp_dir: &Path) -> Result<()> {
        let repo = Git2Repository::init(temp_dir)?;

        // Configure git
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        // Create a test file and commit
        let file_path = temp_dir.join("test.txt");
        let mut file = fs::File::create(&file_path)?;
        writeln!(file, "Hello, world!")?;
        drop(file);

        // Stage and commit
        let mut index = repo.index()?;
        index.add_path(Path::new("test.txt"))?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = repo.signature()?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit #123",
            &tree,
            &[],
        )?;

        Ok(())
    }

    #[test]
    fn test_parse_commits() {
        let temp_dir = TempDir::new().unwrap();
        create_test_repo_with_commits(temp_dir.path()).unwrap();

        let timespan = Timespan::days_back(1);
        let parser = Parser::new(None, timespan);

        let commits = parser.parse_commits(temp_dir.path()).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "Initial commit #123");
        assert_eq!(commits[0].author.email, "test@example.com");
    }

    #[test]
    fn test_author_filter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_repo_with_commits(temp_dir.path()).unwrap();

        let timespan = Timespan::days_back(1);
        let parser = Parser::new(Some("test@example.com".to_string()), timespan);

        let commits = parser.parse_commits(temp_dir.path()).unwrap();
        assert_eq!(commits.len(), 1);

        // Wrong author filter
        let timespan = Timespan::days_back(1);
        let parser = Parser::new(Some("wrong@example.com".to_string()), timespan);
        let commits = parser.parse_commits(temp_dir.path()).unwrap();
        assert_eq!(commits.len(), 0);
    }

    #[test]
    fn test_split_message() {
        let message = "Summary line\n\nBody paragraph 1\n\nBody paragraph 2";
        let (summary, body) = Parser::split_message(message);
        assert_eq!(summary, "Summary line");
        assert!(body.is_some());
        assert!(body.unwrap().contains("Body paragraph"));

        // Single line message
        let message = "Just summary";
        let (summary, body) = Parser::split_message(message);
        assert_eq!(summary, "Just summary");
        assert!(body.is_none());
    }
}
