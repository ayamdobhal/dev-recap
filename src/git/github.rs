use crate::git::GitHubRepo;
use regex::Regex;

/// Extract PR numbers from a commit message
pub fn extract_pr_numbers(message: &str) -> Vec<u32> {
    let mut pr_numbers = Vec::new();

    // Patterns to match:
    // - #123
    // - GH-123
    // - PR#123
    // - Merge pull request #123
    let patterns = vec![
        r"#(\d+)",
        r"GH-(\d+)",
        r"PR#(\d+)",
        r"pull request #(\d+)",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(message) {
                if let Some(num_match) = cap.get(1) {
                    if let Ok(num) = num_match.as_str().parse::<u32>() {
                        if !pr_numbers.contains(&num) {
                            pr_numbers.push(num);
                        }
                    }
                }
            }
        }
    }

    pr_numbers.sort();
    pr_numbers
}

/// Parse GitHub repository information from a remote URL
pub fn parse_github_url(url: &str) -> Option<GitHubRepo> {
    // Handle different GitHub URL formats:
    // - https://github.com/owner/repo.git
    // - git@github.com:owner/repo.git
    // - https://github.com/owner/repo
    // - git://github.com/owner/repo.git

    let url = url.trim();

    // Try HTTPS format
    if let Some(captures) = Regex::new(r"https://github\.com/([^/]+)/([^/.]+)")
        .ok()?
        .captures(url)
    {
        return Some(GitHubRepo {
            owner: captures.get(1)?.as_str().to_string(),
            repo: captures.get(2)?.as_str().trim_end_matches(".git").to_string(),
        });
    }

    // Try SSH format
    if let Some(captures) = Regex::new(r"git@github\.com:([^/]+)/([^/.]+)")
        .ok()?
        .captures(url)
    {
        return Some(GitHubRepo {
            owner: captures.get(1)?.as_str().to_string(),
            repo: captures.get(2)?.as_str().trim_end_matches(".git").to_string(),
        });
    }

    // Try git:// format
    if let Some(captures) = Regex::new(r"git://github\.com/([^/]+)/([^/.]+)")
        .ok()?
        .captures(url)
    {
        return Some(GitHubRepo {
            owner: captures.get(1)?.as_str().to_string(),
            repo: captures.get(2)?.as_str().trim_end_matches(".git").to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pr_numbers() {
        // Test various formats
        assert_eq!(extract_pr_numbers("Fix bug #123"), vec![123]);
        assert_eq!(extract_pr_numbers("Fixes GH-456"), vec![456]);
        assert_eq!(extract_pr_numbers("Closes PR#789"), vec![789]);
        assert_eq!(
            extract_pr_numbers("Merge pull request #101 from user/branch"),
            vec![101]
        );

        // Test multiple PRs in one message
        assert_eq!(
            extract_pr_numbers("Fix #123 and #456"),
            vec![123, 456]
        );

        // Test no PRs
        let empty: Vec<u32> = vec![];
        assert_eq!(extract_pr_numbers("Regular commit message"), empty);

        // Test duplicate PRs (should deduplicate)
        assert_eq!(
            extract_pr_numbers("Fix #123 and close #123"),
            vec![123]
        );
    }

    #[test]
    fn test_parse_github_url_https() {
        let url = "https://github.com/rust-lang/rust.git";
        let repo = parse_github_url(url).unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.repo, "rust");

        // Without .git
        let url = "https://github.com/rust-lang/rust";
        let repo = parse_github_url(url).unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.repo, "rust");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let url = "git@github.com:rust-lang/rust.git";
        let repo = parse_github_url(url).unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.repo, "rust");
    }

    #[test]
    fn test_parse_github_url_git_protocol() {
        let url = "git://github.com/rust-lang/rust.git";
        let repo = parse_github_url(url).unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.repo, "rust");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_none());
        assert!(parse_github_url("not a url").is_none());
        assert!(parse_github_url("").is_none());
    }

    #[test]
    fn test_github_repo_urls() {
        let repo = GitHubRepo {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
        };

        assert_eq!(
            repo.pr_url(123),
            "https://github.com/owner/repo/pull/123"
        );
        assert_eq!(
            repo.commit_url("abc123"),
            "https://github.com/owner/repo/commit/abc123"
        );
    }
}
