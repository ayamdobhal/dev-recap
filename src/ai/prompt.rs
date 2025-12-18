use crate::git::Repository;

/// Generate a prompt for Claude to summarize git commits
pub fn generate_summary_prompt(repo: &Repository) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are helping a developer prepare for Demo Day presentation.\n\n");

    // Repository info
    prompt.push_str(&format!("Repository: {}\n", repo.name));

    if let Some(ref url) = repo.remote_url {
        prompt.push_str(&format!("URL: {}\n", url));
    }

    // Timespan info
    if let (Some(first), Some(last)) = (repo.commits.first(), repo.commits.last()) {
        prompt.push_str(&format!(
            "Timespan: {} to {}\n",
            last.timestamp.format("%Y-%m-%d"),
            first.timestamp.format("%Y-%m-%d")
        ));
    }

    // Statistics
    prompt.push_str(&format!("\nStatistics:\n"));
    prompt.push_str(&format!("- Total commits: {}\n", repo.stats.total_commits));
    prompt.push_str(&format!("- Files changed: {}\n", repo.stats.total_files_changed));
    prompt.push_str(&format!("- Lines added: {}\n", repo.stats.total_insertions));
    prompt.push_str(&format!("- Lines deleted: {}\n", repo.stats.total_deletions));
    prompt.push_str(&format!(
        "- Net lines: {:+}\n",
        repo.stats.net_lines_changed()
    ));

    if repo.stats.pr_count > 0 {
        prompt.push_str(&format!("- Pull requests: {}\n", repo.stats.pr_count));
    }

    // Commits
    prompt.push_str(&format!("\nCommits ({}):\n", repo.commits.len()));
    for (i, commit) in repo.commits.iter().take(50).enumerate() {
        // Limit to first 50 commits to avoid token limits
        prompt.push_str(&format!("{}. {} - {}\n", i + 1, commit.short_hash, commit.summary));

        // Add PR links if available
        if !commit.pr_numbers.is_empty() {
            let pr_refs: Vec<String> = commit
                .pr_numbers
                .iter()
                .map(|n| format!("#{}", n))
                .collect();
            prompt.push_str(&format!("   PRs: {}\n", pr_refs.join(", ")));
        }

        // Add file changes (limited)
        if !commit.files_changed.is_empty() {
            let file_count = commit.files_changed.len();
            let files: Vec<&String> = commit.files_changed.iter().take(5).collect();
            let file_list = files
                .iter()
                .map(|f| f.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            if file_count > 5 {
                prompt.push_str(&format!(
                    "   Files: {} (+{} more)\n",
                    file_list,
                    file_count - 5
                ));
            } else {
                prompt.push_str(&format!("   Files: {}\n", file_list));
            }
        }
    }

    if repo.commits.len() > 50 {
        prompt.push_str(&format!(
            "\n(Showing first 50 of {} commits)\n",
            repo.commits.len()
        ));
    }

    // Instructions
    prompt.push_str("\nPlease provide:\n");
    prompt.push_str("1. A concise summary of the work done (2-3 paragraphs)\n");
    prompt.push_str("2. Key achievements (3-5 bullet points)\n");
    prompt.push_str("3. Tips for presenting this work in a screenshare demo (3-5 tips)\n\n");
    prompt.push_str("Format your response EXACTLY as follows:\n\n");
    prompt.push_str("## Summary\n");
    prompt.push_str("[Your 2-3 paragraph summary here]\n\n");
    prompt.push_str("## Key Achievements\n");
    prompt.push_str("- [Achievement 1]\n");
    prompt.push_str("- [Achievement 2]\n");
    prompt.push_str("- [Achievement 3]\n\n");
    prompt.push_str("## Presentation Tips\n");
    prompt.push_str("1. [Tip 1]\n");
    prompt.push_str("2. [Tip 2]\n");
    prompt.push_str("3. [Tip 3]\n");

    prompt
}

/// Parse Claude's response into structured data
pub fn parse_response(response: &str) -> (String, Vec<String>, Vec<String>) {
    let mut achievements = Vec::new();
    let mut tips = Vec::new();

    let mut current_section = None;
    let mut summary_lines = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("## Summary") {
            current_section = Some("summary");
            continue;
        } else if trimmed.starts_with("## Key Achievements") {
            current_section = Some("achievements");
            continue;
        } else if trimmed.starts_with("## Presentation Tips") {
            current_section = Some("tips");
            continue;
        }

        match current_section {
            Some("summary") => {
                if !trimmed.is_empty() && !trimmed.starts_with("##") {
                    summary_lines.push(trimmed.to_string());
                }
            }
            Some("achievements") => {
                if let Some(achievement) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
                    achievements.push(achievement.trim().to_string());
                }
            }
            Some("tips") => {
                // Match numbered lists: "1. ", "2. ", etc.
                if let Some(tip) = trimmed.chars().next() {
                    if tip.is_numeric() && trimmed.contains(". ") {
                        if let Some(content) = trimmed.split(". ").nth(1) {
                            tips.push(content.trim().to_string());
                        }
                    }
                }
            }
            Some(_) | None => {}
        }
    }

    let summary = summary_lines.join(" ");

    (summary, achievements, tips)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{Author, Commit, RepoStats};
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_repo() -> Repository {
        let commit = Commit {
            hash: "abc123".to_string(),
            short_hash: "abc123".to_string(),
            author: Author {
                name: "Test".to_string(),
                email: "test@example.com".to_string(),
            },
            timestamp: Utc::now(),
            message: "Test commit".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            files_changed: vec!["file1.rs".to_string()],
            insertions: 10,
            deletions: 5,
            pr_numbers: vec![123],
        };

        Repository {
            path: PathBuf::from("/test"),
            name: "test-repo".to_string(),
            remote_url: Some("https://github.com/test/repo".to_string()),
            github_info: None,
            commits: vec![commit.clone()],
            stats: RepoStats::from_commits(&vec![commit]),
        }
    }

    #[test]
    fn test_generate_summary_prompt() {
        let repo = create_test_repo();
        let prompt = generate_summary_prompt(&repo);

        assert!(prompt.contains("Repository: test-repo"));
        assert!(prompt.contains("Statistics:"));
        assert!(prompt.contains("Commits (1):"));
        assert!(prompt.contains("Test commit"));
        assert!(prompt.contains("## Summary"));
        assert!(prompt.contains("## Key Achievements"));
        assert!(prompt.contains("## Presentation Tips"));
    }

    #[test]
    fn test_parse_response() {
        let response = r#"
## Summary
This is a test summary.
It has multiple lines.

## Key Achievements
- Achievement 1
- Achievement 2
- Achievement 3

## Presentation Tips
1. Tip number one
2. Tip number two
3. Tip number three
"#;

        let (summary, achievements, tips) = parse_response(response);

        assert!(summary.contains("test summary"));
        assert_eq!(achievements.len(), 3);
        assert_eq!(achievements[0], "Achievement 1");
        assert_eq!(tips.len(), 3);
        assert_eq!(tips[0], "Tip number one");
    }

    #[test]
    fn test_parse_response_with_asterisk_bullets() {
        let response = r#"
## Summary
Test summary

## Key Achievements
* Achievement with asterisk
* Another achievement

## Presentation Tips
1. First tip
"#;

        let (_summary, achievements, tips) = parse_response(response);

        assert_eq!(achievements.len(), 2);
        assert_eq!(achievements[0], "Achievement with asterisk");
        assert_eq!(tips.len(), 1);
    }
}
