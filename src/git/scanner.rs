use crate::error::Result;
use git2::Repository as Git2Repository;
use std::fs;
use std::path::{Path, PathBuf};

/// Scanner for discovering git repositories
pub struct Scanner {
    /// Patterns to exclude from scanning
    exclude_patterns: Vec<String>,
    /// Maximum directory depth (None = unlimited)
    max_depth: Option<u32>,
}

impl Scanner {
    /// Create a new scanner
    pub fn new(exclude_patterns: Vec<String>, max_depth: Option<u32>) -> Self {
        Self {
            exclude_patterns,
            max_depth,
        }
    }

    /// Scan a directory for git repositories
    pub fn scan(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut repos = Vec::new();
        self.scan_recursive(path, 0, &mut repos)?;
        Ok(repos)
    }

    /// Recursively scan directories
    fn scan_recursive(&self, path: &Path, depth: u32, repos: &mut Vec<PathBuf>) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if depth >= max_depth {
                return Ok(());
            }
        }

        // Check if this is a git repository
        let is_repo = self.is_git_repository(path);
        if is_repo {
            repos.push(path.to_path_buf());
            // Continue scanning inside to find submodules
        }

        // Read directory entries
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => {
                // Skip directories we can't read (permission denied, etc.)
                return Ok(());
            }
        };

        // Scan subdirectories
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();

            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }

            // Get directory name
            let dir_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // Skip excluded patterns
            if self.should_exclude(&dir_name) {
                continue;
            }

            // Skip hidden directories (except .git when checking for repos)
            if dir_name.starts_with('.') && dir_name != ".git" {
                continue;
            }

            // Recursively scan subdirectory
            self.scan_recursive(&path, depth + 1, repos)?;
        }

        Ok(())
    }

    /// Check if a path is a git repository
    fn is_git_repository(&self, path: &Path) -> bool {
        // Check if .git directory or file exists
        let git_path = path.join(".git");
        if !git_path.exists() {
            return false;
        }

        // Verify it's a valid git repository
        Git2Repository::open(path).is_ok()
    }

    /// Check if a directory name should be excluded
    fn should_exclude(&self, name: &str) -> bool {
        for pattern in &self.exclude_patterns {
            if name == pattern || name.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Get repository name from path
    pub fn get_repo_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Get remote URL from a git repository
    pub fn get_remote_url(path: &Path) -> Option<String> {
        let repo = Git2Repository::open(path).ok()?;
        let remote = repo.find_remote("origin").ok()?;
        remote.url().map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_git_repo(path: &Path) -> Result<()> {
        // Initialize a git repository
        Git2Repository::init(path)?;
        Ok(())
    }

    #[test]
    fn test_scanner_finds_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_path).unwrap();
        create_test_git_repo(&repo_path).unwrap();

        let scanner = Scanner::new(vec![], None);
        let repos = scanner.scan(temp_dir.path()).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], repo_path);
    }

    #[test]
    fn test_scanner_excludes_patterns() {
        let temp_dir = TempDir::new().unwrap();

        // Create repos
        let repo1 = temp_dir.path().join("good-repo");
        let repo2 = temp_dir.path().join("node_modules").join("bad-repo");
        fs::create_dir_all(&repo1).unwrap();
        fs::create_dir_all(&repo2).unwrap();
        create_test_git_repo(&repo1).unwrap();
        create_test_git_repo(&repo2).unwrap();

        let scanner = Scanner::new(vec!["node_modules".to_string()], None);
        let repos = scanner.scan(temp_dir.path()).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], repo1);
    }

    #[test]
    fn test_scanner_respects_max_depth() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested repos
        let shallow = temp_dir.path().join("shallow");
        let deep = temp_dir.path().join("a").join("b").join("c").join("deep");
        fs::create_dir_all(&shallow).unwrap();
        fs::create_dir_all(&deep).unwrap();
        create_test_git_repo(&shallow).unwrap();
        create_test_git_repo(&deep).unwrap();

        // Scan with depth limit
        let scanner = Scanner::new(vec![], Some(2));
        let repos = scanner.scan(temp_dir.path()).unwrap();

        // Should only find the shallow repo
        assert_eq!(repos.len(), 1);
        assert!(repos[0].ends_with("shallow"));
    }

    #[test]
    fn test_get_repo_name() {
        let path = PathBuf::from("/path/to/my-repo");
        assert_eq!(Scanner::get_repo_name(&path), "my-repo");
    }

    #[test]
    fn test_should_exclude() {
        let scanner = Scanner::new(
            vec!["node_modules".to_string(), "target".to_string()],
            None,
        );

        assert!(scanner.should_exclude("node_modules"));
        assert!(scanner.should_exclude("target"));
        assert!(!scanner.should_exclude("src"));
    }

    #[test]
    fn test_scanner_finds_submodules() {
        let temp_dir = TempDir::new().unwrap();

        // Create main repo with a submodule inside
        let main_repo = temp_dir.path().join("main-repo");
        let submodule = main_repo.join("submodules").join("sub-repo");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&submodule).unwrap();
        create_test_git_repo(&main_repo).unwrap();
        create_test_git_repo(&submodule).unwrap();

        let scanner = Scanner::new(vec![], None);
        let repos = scanner.scan(temp_dir.path()).unwrap();

        // Should find both repos
        assert_eq!(repos.len(), 2);
        assert!(repos.contains(&main_repo));
        assert!(repos.contains(&submodule));
    }
}
