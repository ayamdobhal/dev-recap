use crate::ai::cache::SummaryCache;
use crate::ai::claude::ClaudeClient;
use crate::ai::prompt::{generate_summary_prompt, parse_response};
use crate::ai::Summary;
use crate::config::Config;
use crate::error::{DevRecapError, Result};
use crate::git::github::parse_github_url;
use crate::git::parser::Parser;
use crate::git::scanner::Scanner;
use crate::git::{RepoStats, Repository, Timespan};
use std::path::{Path, PathBuf};

/// Orchestrator for coordinating the analysis workflow
pub struct Orchestrator {
    config: Config,
    scanner: Scanner,
    cache: Option<SummaryCache>,
    claude_client: ClaudeClient,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new(config: Config) -> Result<Self> {
        let scanner = Scanner::new(
            config.exclude_patterns.clone(),
            config.max_scan_depth,
        );

        let cache = if config.cache_enabled {
            Some(SummaryCache::from_config(&config)?)
        } else {
            None
        };

        let claude_client = ClaudeClient::new(config.claude_api_key.clone())?;

        Ok(Self {
            config,
            scanner,
            cache,
            claude_client,
        })
    }

    /// Scan a directory for repositories
    pub fn scan_repositories(&self, path: &Path) -> Result<Vec<PathBuf>> {
        self.scanner.scan(path)
    }

    /// Analyze a single repository
    pub fn analyze_repository(
        &self,
        repo_path: &Path,
        author_email: Option<&str>,
        timespan: &Timespan,
    ) -> Result<Repository> {
        // Parse commits
        let parser = Parser::new(author_email.map(String::from), timespan.clone());
        let commits = parser.parse_commits(repo_path)?;

        if commits.is_empty() {
            return Err(DevRecapError::NoCommitsFound {
                author: author_email.unwrap_or("any").to_string(),
            });
        }

        // Calculate statistics
        let stats = RepoStats::from_commits(&commits);

        // Get repository info
        let name = Scanner::get_repo_name(repo_path);
        let remote_url = Scanner::get_remote_url(repo_path);
        let github_info = remote_url
            .as_ref()
            .and_then(|url| parse_github_url(url));

        Ok(Repository {
            path: repo_path.to_path_buf(),
            name,
            remote_url,
            github_info,
            commits,
            stats,
        })
    }

    /// Generate summary for a repository using AI
    pub async fn generate_summary(&self, repo: &Repository) -> Result<Summary> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            let commit_hashes: Vec<String> = repo
                .commits
                .iter()
                .map(|c| c.hash.clone())
                .collect();

            let cache_key = SummaryCache::generate_key(
                &repo.path.to_string_lossy(),
                &commit_hashes,
            );

            // Try to get from cache
            if let Some(cached_summary) = cache.get(&cache_key)? {
                return Ok(cached_summary);
            }

            // Generate new summary
            let summary = self.generate_summary_uncached(repo).await?;

            // Store in cache
            cache.set(&cache_key, summary.clone())?;

            Ok(summary)
        } else {
            // No cache, generate directly
            self.generate_summary_uncached(repo).await
        }
    }

    /// Generate summary without using cache
    async fn generate_summary_uncached(&self, repo: &Repository) -> Result<Summary> {
        // Generate prompt
        let prompt = generate_summary_prompt(repo);

        // Call Claude API
        let response = self.claude_client.generate_summary(prompt).await?;

        // Parse response
        let (work_summary, key_achievements, presentation_tips) = parse_response(&response);

        Ok(Summary::new(
            repo.name.clone(),
            work_summary,
            key_achievements,
            presentation_tips,
        ))
    }

    /// Analyze multiple repositories
    pub async fn analyze_repositories(
        &self,
        repo_paths: &[PathBuf],
        author_email: Option<&str>,
        timespan: &Timespan,
    ) -> Vec<(Repository, Result<Summary>)> {
        let mut results = Vec::new();

        for repo_path in repo_paths {
            // Analyze repository
            let repo_result = self.analyze_repository(repo_path, author_email, timespan);

            match repo_result {
                Ok(repo) => {
                    // Generate summary
                    let summary_result = self.generate_summary(&repo).await;
                    results.push((repo, summary_result));
                }
                Err(e) => {
                    // Create a minimal repository for error reporting
                    let repo = Repository {
                        path: repo_path.clone(),
                        name: Scanner::get_repo_name(repo_path),
                        remote_url: None,
                        github_info: None,
                        commits: vec![],
                        stats: RepoStats::default(),
                    };
                    results.push((repo, Err(e)));
                }
            }
        }

        results
    }

    /// Get a reference to the config
    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
            default_author_email: Some("test@example.com".to_string()),
            claude_api_key: "sk-ant-test-key".to_string(),
            default_timespan_days: 14,
            exclude_patterns: vec!["node_modules".to_string()],
            max_scan_depth: None,
            cache_enabled: false,
            cache_ttl_hours: 168,
            github_token: None,
        }
    }

    fn create_test_repo_with_commits(temp_dir: &Path) -> Result<()> {
        let repo = git2::Repository::init(temp_dir)?;

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
            "Initial commit",
            &tree,
            &[],
        )?;

        Ok(())
    }

    #[test]
    fn test_orchestrator_creation() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();
        assert!(orchestrator.cache.is_none());
    }

    #[test]
    fn test_scan_repositories() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_path).unwrap();
        create_test_repo_with_commits(&repo_path).unwrap();

        let repos = orchestrator.scan_repositories(temp_dir.path()).unwrap();
        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn test_analyze_repository() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();

        let temp_dir = TempDir::new().unwrap();
        create_test_repo_with_commits(temp_dir.path()).unwrap();

        let timespan = Timespan::days_back(1);
        let repo = orchestrator
            .analyze_repository(temp_dir.path(), Some("test@example.com"), &timespan)
            .unwrap();

        assert_eq!(repo.commits.len(), 1);
        assert_eq!(repo.stats.total_commits, 1);
    }

    #[test]
    fn test_analyze_repository_no_commits() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();

        let temp_dir = TempDir::new().unwrap();
        create_test_repo_with_commits(temp_dir.path()).unwrap();

        let timespan = Timespan::days_back(1);
        // Use wrong author email
        let result = orchestrator.analyze_repository(
            temp_dir.path(),
            Some("wrong@example.com"),
            &timespan,
        );

        assert!(result.is_err());
    }
}
