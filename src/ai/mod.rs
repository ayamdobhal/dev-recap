pub mod cache;
pub mod claude;
pub mod prompt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// AI-generated summary for a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Repository name
    pub repository: String,
    /// Work summary in markdown format
    pub work_summary: String,
    /// Key achievements
    pub key_achievements: Vec<String>,
    /// Presentation tips
    pub presentation_tips: Vec<String>,
    /// When this summary was generated
    pub generated_at: DateTime<Utc>,
}

impl Summary {
    /// Create a new summary
    pub fn new(
        repository: String,
        work_summary: String,
        key_achievements: Vec<String>,
        presentation_tips: Vec<String>,
    ) -> Self {
        Self {
            repository,
            work_summary,
            key_achievements,
            presentation_tips,
            generated_at: Utc::now(),
        }
    }

    /// Format summary as markdown
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", self.repository));
        output.push_str("## Summary\n\n");
        output.push_str(&self.work_summary);
        output.push_str("\n\n");

        if !self.key_achievements.is_empty() {
            output.push_str("## Key Achievements\n\n");
            for achievement in &self.key_achievements {
                output.push_str(&format!("- {}\n", achievement));
            }
            output.push_str("\n");
        }

        if !self.presentation_tips.is_empty() {
            output.push_str("## Presentation Tips\n\n");
            for (i, tip) in self.presentation_tips.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, tip));
            }
            output.push_str("\n");
        }

        output.push_str(&format!(
            "*Generated at: {}*\n",
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_creation() {
        let summary = Summary::new(
            "test-repo".to_string(),
            "Test summary".to_string(),
            vec!["Achievement 1".to_string()],
            vec!["Tip 1".to_string()],
        );

        assert_eq!(summary.repository, "test-repo");
        assert_eq!(summary.work_summary, "Test summary");
        assert_eq!(summary.key_achievements.len(), 1);
        assert_eq!(summary.presentation_tips.len(), 1);
    }

    #[test]
    fn test_summary_to_markdown() {
        let summary = Summary::new(
            "test-repo".to_string(),
            "Test summary".to_string(),
            vec!["Achievement 1".to_string(), "Achievement 2".to_string()],
            vec!["Tip 1".to_string()],
        );

        let markdown = summary.to_markdown();
        assert!(markdown.contains("# test-repo"));
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("## Key Achievements"));
        assert!(markdown.contains("## Presentation Tips"));
        assert!(markdown.contains("- Achievement 1"));
        assert!(markdown.contains("1. Tip 1"));
    }
}
