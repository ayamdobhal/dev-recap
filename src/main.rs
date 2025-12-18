mod ai;
mod cli;
mod config;
mod error;
mod git;
mod orchestrator;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use error::Result;
use git::Timespan;
use indicatif::{ProgressBar, ProgressStyle};
use orchestrator::Orchestrator;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Validate CLI arguments
    if let Err(e) = cli.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Handle subcommands
    if let Some(command) = &cli.command {
        return handle_command(command);
    }

    // Load or create config
    let config = if let Some(config_path) = &cli.config {
        Config::load_from(config_path)?
    } else {
        Config::load_or_create_default()?
    };

    // Apply CLI overrides to config
    let config = apply_cli_overrides(config, &cli);

    // Verify API key is available (from env or config)
    if let Err(e) = config.get_api_key() {
        eprintln!("Error: {}", e);
        eprintln!("\nPlease either:");
        eprintln!("  1. Set the ANTHROPIC_AUTH_TOKEN environment variable");
        eprintln!("  2. Add claude_api_key to your config file at: {}",
            Config::default_config_path()?.display());
        std::process::exit(1);
    }

    // Run main analysis
    run_analysis(config, &cli).await
}

async fn run_analysis(config: Config, cli: &Cli) -> Result<()> {
    println!("dev-recap v{}", env!("CARGO_PKG_VERSION"));
    println!("AI-powered git commit summarizer for Demo Day presentations\n");

    // Interactive mode: prompt for missing values
    let scan_path = if let Some(ref path) = cli.path {
        path.clone()
    } else {
        let default_path = env::current_dir().expect("Failed to get current directory");
        prompt_with_default("Scan path", &default_path.display().to_string())?
            .parse()
            .unwrap_or(default_path)
    };

    // Prompt for author email(s)
    let author_emails = if cli.team {
        // Team mode: get multiple authors
        if let Some(ref authors) = cli.authors {
            authors.clone()
        } else {
            // Interactive mode: prompt for authors
            let input = prompt_required("Author emails (comma-separated)")?;
            input.split(',').map(|s| s.trim().to_string()).collect()
        }
    } else {
        // Single author mode
        let author_email = if let Some(ref email) = cli.author {
            email.clone()
        } else if let Some(ref default_email) = config.default_author_email {
            prompt_with_default("Author email", default_email)?
        } else {
            // Try to get from git config
            let git_email = get_git_user_email();
            if let Some(ref email) = git_email {
                prompt_with_default("Author email", email)?
            } else {
                prompt_required("Author email")?
            }
        };
        vec![author_email]
    };

    // Prompt for timespan
    let (timespan, timespan_desc) = if cli.since.is_some() || cli.until.is_some() {
        // Use --since/--until for date range
        let since_str = cli.since.as_deref().unwrap_or("1970-01-01");
        let until_str = cli.until.as_deref().unwrap_or_else(|| {
            // Default to today
            chrono::Utc::now().format("%Y-%m-%d").to_string().leak()
        });

        let start = chrono::NaiveDate::parse_from_str(since_str, "%Y-%m-%d")
            .map_err(|_| error::DevRecapError::Other(format!("Invalid date format for --since: {}", since_str)))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| error::DevRecapError::Other("Invalid time".to_string()))?
            .and_utc();

        let end = chrono::NaiveDate::parse_from_str(until_str, "%Y-%m-%d")
            .map_err(|_| error::DevRecapError::Other(format!("Invalid date format for --until: {}", until_str)))?
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| error::DevRecapError::Other("Invalid time".to_string()))?
            .and_utc();

        let timespan = Timespan::from_dates(start, end);
        let desc = format!("{} to {}", since_str, until_str);
        (timespan, desc)
    } else {
        // Use --days for days back
        let days = if let Some(d) = cli.days {
            d
        } else {
            let default_days = config.default_timespan_days;
            let input = prompt_with_default("Days back", &default_days.to_string())?;
            input.parse().unwrap_or(default_days)
        };

        let timespan = Timespan::days_back(days);
        let desc = format!("{} days back", days);
        (timespan, desc)
    };

    println!("\n{}", "=".repeat(60));
    println!("Scanning: {}", scan_path.display());
    if author_emails.len() == 1 {
        println!("Author: {}", author_emails[0]);
    } else {
        println!("Authors: {}", author_emails.join(", "));
    }
    println!("Timespan: {}", timespan_desc);
    println!("{}\n", "=".repeat(60));

    // Create orchestrator
    let orchestrator = Orchestrator::new(config)?;

    // Scan for repositories
    let scan_spinner = ProgressBar::new_spinner();
    scan_spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    scan_spinner.set_message("Scanning for git repositories...");
    scan_spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let repos = orchestrator.scan_repositories(&scan_path)?;

    scan_spinner.finish_with_message(format!("Found {} repositories", repos.len()));

    if repos.is_empty() {
        println!("No git repositories found.");
        return Ok(());
    }

    println!();

    // Analyze repositories
    let progress = ProgressBar::new(repos.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("Analyzing repositories...");

    let mut results = Vec::new();
    for repo_path in &repos {
        // Update progress message with current repo
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        progress.set_message(format!("Analyzing {}", repo_name));

        // Analyze single repository
        // In team mode, analyze all commits; in single mode, filter by author
        let author_filter = if cli.team {
            None // Team mode: get all commits
        } else {
            Some(author_emails[0].as_str()) // Single author mode
        };
        let repo_result = orchestrator.analyze_repository(repo_path, author_filter, &timespan);

        match repo_result {
            Ok(repo) => {
                if cli.dry_run {
                    // Dry run: skip API call, create dummy success result
                    use crate::ai::Summary;
                    let summary = Summary::new(
                        repo.name.clone(),
                        format!("[Dry run] Would analyze {} commits", repo.stats.total_commits),
                        vec![format!("{} files changed", repo.stats.total_files_changed)],
                        vec![],
                    );
                    results.push((repo, Ok(summary)));
                } else {
                    // Generate summary
                    let summary_result = orchestrator.generate_summary(&repo).await;
                    results.push((repo, summary_result));
                }
            }
            Err(e) => {
                // Create a minimal repository for error reporting
                let repo = git::Repository {
                    path: repo_path.clone(),
                    name: git::scanner::Scanner::get_repo_name(repo_path),
                    remote_url: None,
                    github_info: None,
                    commits: vec![],
                    stats: git::RepoStats::default(),
                };
                results.push((repo, Err(e)));
            }
        }

        progress.inc(1);
    }

    progress.finish_with_message(if cli.dry_run {
        "Dry run complete"
    } else {
        "Analysis complete"
    });

    // Build markdown output
    let mut markdown_output = String::new();
    markdown_output.push_str(&format!("# Dev Recap\n\n"));
    markdown_output.push_str(&format!("**Scan Path:** {}\n", scan_path.display()));
    if author_emails.len() == 1 {
        markdown_output.push_str(&format!("**Author:** {}\n", author_emails[0]));
    } else {
        markdown_output.push_str(&format!("**Authors:** {}\n", author_emails.join(", ")));
    }
    markdown_output.push_str(&format!("**Timespan:** {}\n\n", timespan_desc));
    markdown_output.push_str(&format!("---\n\n"));

    for (repo, summary_result) in &results {
        markdown_output.push_str(&format!("## Repository: {}\n\n", repo.name));
        markdown_output.push_str(&format!("**Path:** {}\n\n", repo.path.display()));

        // Add verbose information if requested
        if cli.verbose >= 1 && !repo.commits.is_empty() {
            markdown_output.push_str(&format!("**Stats:**\n"));
            markdown_output.push_str(&format!("- Total commits: {}\n", repo.stats.total_commits));
            markdown_output.push_str(&format!("- Files changed: {}\n", repo.stats.total_files_changed));
            markdown_output.push_str(&format!("- Insertions: +{}\n", repo.stats.total_insertions));
            markdown_output.push_str(&format!("- Deletions: -{}\n", repo.stats.total_deletions));
            markdown_output.push_str(&format!("- Net change: {}\n\n", repo.stats.net_lines_changed()));
        }

        // Add commit list if verbose >= 2
        if cli.verbose >= 2 && !repo.commits.is_empty() {
            markdown_output.push_str(&format!("**Commits:**\n"));
            for commit in &repo.commits {
                markdown_output.push_str(&format!("- `{}` {}\n", commit.short_hash, commit.summary));
            }
            markdown_output.push_str("\n");
        }

        match summary_result {
            Ok(summary) => {
                markdown_output.push_str(&summary.to_markdown());
                markdown_output.push_str("\n\n");
            }
            Err(e) => {
                markdown_output.push_str(&format!("**Error:** {}\n\n", e));
            }
        }

        markdown_output.push_str("---\n\n");
    }

    // Write to file if --output is specified
    if let Some(output_path) = &cli.output {
        std::fs::write(output_path, &markdown_output)?;
        println!("\n✓ Results written to: {}", output_path.display());
    } else {
        // Display results to stdout
        println!("\n{}\n", "=".repeat(60));
        for (repo, summary_result) in results {
            println!("Repository: {}", repo.name);
            println!("Path: {}", repo.path.display());

            // Add verbose information if requested
            if cli.verbose >= 1 && !repo.commits.is_empty() {
                println!("\nStats:");
                println!("  Total commits: {}", repo.stats.total_commits);
                println!("  Files changed: {}", repo.stats.total_files_changed);
                println!("  Insertions: +{}", repo.stats.total_insertions);
                println!("  Deletions: -{}", repo.stats.total_deletions);
                println!("  Net change: {}", repo.stats.net_lines_changed());
            }

            // Add commit list if verbose >= 2
            if cli.verbose >= 2 && !repo.commits.is_empty() {
                println!("\nCommits:");
                for commit in &repo.commits {
                    println!("  - {} {}", commit.short_hash, commit.summary);
                }
            }

            match summary_result {
                Ok(summary) => {
                    println!("\n{}", summary.to_markdown());
                }
                Err(e) => {
                    println!("\n❌ Error: {}", e);
                }
            }

            println!("\n{}\n", "-".repeat(60));
        }
    }

    Ok(())
}

fn handle_command(command: &Commands) -> Result<()> {
    match command {
        Commands::Init { force } => {
            let config_path = Config::default_config_path()?;

            if config_path.exists() && !force {
                eprintln!(
                    "Config file already exists at: {}",
                    config_path.display()
                );
                eprintln!("Use --force to overwrite");
                std::process::exit(1);
            }

            Config::create_default()?;
            println!("✓ Created config file at: {}", config_path.display());
            println!("\nTo authenticate with Claude, either:");
            println!("  1. Set the ANTHROPIC_AUTH_TOKEN environment variable");
            println!("  2. Add claude_api_key to the config file:");
            println!("     claude_api_key = \"sk-ant-YOUR_KEY_HERE\"");
        }
        Commands::Config => {
            let config = Config::load_or_create_default()?;
            let toml_str = toml::to_string_pretty(&config)?;
            println!("Current configuration:\n");
            println!("{}", toml_str);
        }
        Commands::ClearCache => {
            let cache_dir = Config::default_cache_dir()?;
            if cache_dir.exists() {
                std::fs::remove_dir_all(&cache_dir)?;
                println!("✓ Cache cleared: {}", cache_dir.display());
            } else {
                println!("Cache directory does not exist");
            }
        }
        Commands::CacheStats => {
            let cache_dir = Config::default_cache_dir()?;
            if !cache_dir.exists() {
                println!("Cache directory does not exist");
            } else {
                println!("Cache directory: {}", cache_dir.display());

                // Try to load cache and show stats
                if let Ok(cache) = ai::cache::SummaryCache::new(&cache_dir, 0) {
                    let stats = cache.stats();
                    println!("Total entries: {}", stats.total_entries);
                    println!("Database size: {}", stats.format_size());
                } else {
                    println!("Could not open cache database");
                }
            }
        }
    }
    Ok(())
}

/// Prompt user with a default value (press Enter to accept default)
fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

/// Prompt user for required value (cannot be empty)
fn prompt_required(prompt: &str) -> Result<String> {
    loop {
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        if !input.is_empty() {
            return Ok(input);
        }
        eprintln!("This field is required. Please enter a value.");
    }
}

/// Try to get user email from git config
fn get_git_user_email() -> Option<String> {
    use std::process::Command;

    Command::new("git")
        .args(&["config", "--get", "user.email"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn apply_cli_overrides(mut config: Config, cli: &Cli) -> Config {
    // Override author if provided
    if let Some(ref author) = cli.author {
        config.default_author_email = Some(author.clone());
    }

    // Override timespan if provided
    if let Some(days) = cli.days {
        config.default_timespan_days = days;
    }

    // Override cache setting
    if cli.no_cache {
        config.cache_enabled = false;
    }

    // Override max depth
    if let Some(depth) = cli.max_depth {
        config.max_scan_depth = Some(depth);
    }

    config
}
