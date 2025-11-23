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

    // Prompt for author email
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

    // Prompt for timespan
    let days = if let Some(d) = cli.days {
        d
    } else {
        let default_days = config.default_timespan_days;
        let input = prompt_with_default("Days back", &default_days.to_string())?;
        input.parse().unwrap_or(default_days)
    };

    let timespan = Timespan::days_back(days);

    println!("\n{}", "=".repeat(60));
    println!("Scanning: {}", scan_path.display());
    println!("Author: {}", author_email);
    println!("Timespan: {} days back", days);
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
        let repo_result = orchestrator.analyze_repository(repo_path, Some(&author_email), &timespan);

        match repo_result {
            Ok(repo) => {
                // Generate summary
                let summary_result = orchestrator.generate_summary(&repo).await;
                results.push((repo, summary_result));
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

    progress.finish_with_message("Analysis complete");

    // Display results
    println!("\n{}\n", "=".repeat(60));
    for (repo, summary_result) in results {
        println!("Repository: {}", repo.name);
        println!("Path: {}", repo.path.display());

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
