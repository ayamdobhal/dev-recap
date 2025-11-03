use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dev-recap")]
#[command(author, version, about, long_about = None)]
#[command(
    about = "AI-powered git commit summarizer for Demo Day presentations",
    long_about = "dev-recap analyzes your git commit history and generates AI-powered summaries \
                  perfect for Demo Day presentations. It can scan multiple repositories, \
                  filter by author and timespan, and provide presentation tips."
)]
pub struct Cli {
    /// Path to scan for git repositories (default: current directory)
    #[arg(short, long, value_name = "DIR")]
    pub path: Option<PathBuf>,

    /// Author email to filter commits
    #[arg(short, long)]
    pub author: Option<String>,

    /// Number of days to look back
    #[arg(short, long, value_name = "DAYS")]
    pub days: Option<u32>,

    /// Start date (YYYY-MM-DD format)
    #[arg(long)]
    pub since: Option<String>,

    /// End date (YYYY-MM-DD format)
    #[arg(long)]
    pub until: Option<String>,

    /// Path to config file (default: ~/.config/dev-recap/config.toml)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Output file path (markdown format)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Run in non-interactive mode (skip TUI)
    #[arg(long)]
    pub non_interactive: bool,

    /// Disable caching
    #[arg(long)]
    pub no_cache: bool,

    /// Dry run - show what would be analyzed without making API calls
    #[arg(long)]
    pub dry_run: bool,

    /// Team mode - analyze multiple authors
    #[arg(long)]
    pub team: bool,

    /// Comma-separated list of author emails (for team mode)
    #[arg(long, value_delimiter = ',')]
    pub authors: Option<Vec<String>>,

    /// Maximum directory scan depth
    #[arg(long)]
    pub max_depth: Option<u32>,

    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize configuration file
    Init {
        /// Overwrite existing config file
        #[arg(long)]
        force: bool,
    },

    /// Show current configuration
    Config,

    /// Clear the cache
    ClearCache,

    /// Show cache statistics
    CacheStats,
}

impl Cli {
    /// Check if the CLI is in non-interactive mode
    pub fn is_non_interactive(&self) -> bool {
        self.non_interactive
            || self.output.is_some()
            || self.dry_run
            || self.command.is_some()
    }

    /// Validate CLI arguments
    pub fn validate(&self) -> Result<(), String> {
        // Can't specify both --days and --since/--until
        if self.days.is_some() && (self.since.is_some() || self.until.is_some()) {
            return Err(
                "Cannot specify both --days and --since/--until. Choose one.".to_string()
            );
        }

        // If --authors is provided, --team should be enabled
        if self.authors.is_some() && !self.team {
            return Err("--authors requires --team flag".to_string());
        }

        // Team mode requires either --authors or interactive mode
        if self.team && self.is_non_interactive() && self.authors.is_none() {
            return Err("Team mode in non-interactive mode requires --authors".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_basic() {
        let cli = Cli::parse_from(vec!["dev-recap"]);
        assert!(cli.path.is_none());
        assert!(cli.author.is_none());
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parse_with_options() {
        let cli = Cli::parse_from(vec![
            "dev-recap",
            "--author",
            "test@example.com",
            "--days",
            "30",
            "--output",
            "summary.md",
        ]);
        assert_eq!(cli.author, Some("test@example.com".to_string()));
        assert_eq!(cli.days, Some(30));
        assert!(cli.output.is_some());
    }

    #[test]
    fn test_cli_team_mode() {
        let cli = Cli::parse_from(vec![
            "dev-recap",
            "--team",
            "--authors",
            "alice@example.com,bob@example.com",
        ]);
        assert!(cli.team);
        assert_eq!(
            cli.authors,
            Some(vec![
                "alice@example.com".to_string(),
                "bob@example.com".to_string()
            ])
        );
    }

    #[test]
    fn test_cli_init_command() {
        let cli = Cli::parse_from(vec!["dev-recap", "init"]);
        assert!(matches!(cli.command, Some(Commands::Init { force: false })));
    }

    #[test]
    fn test_cli_validation_days_and_since() {
        let cli = Cli::parse_from(vec![
            "dev-recap",
            "--days",
            "30",
            "--since",
            "2025-01-01",
        ]);
        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_validation_authors_without_team() {
        let cli = Cli::parse_from(vec![
            "dev-recap",
            "--authors",
            "alice@example.com",
        ]);
        assert!(cli.validate().is_err());
    }
}
