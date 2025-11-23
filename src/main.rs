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
use orchestrator::Orchestrator;
use std::env;

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

    // Run main analysis
    run_analysis(config, &cli).await
}

async fn run_analysis(config: Config, cli: &Cli) -> Result<()> {
    println!("dev-recap v{}", env!("CARGO_PKG_VERSION"));
    println!("AI-powered git commit summarizer for Demo Day presentations\n");

    // Determine scan path
    let scan_path = cli
        .path
        .as_ref()
        .map(|p| p.clone())
        .unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    // Determine author email (clone to avoid borrow issues)
    let author_email = cli
        .author
        .clone()
        .or_else(|| config.default_author_email.clone());

    if author_email.is_none() {
        eprintln!("Error: No author email specified.");
        eprintln!("Provide via --author flag or set default_author_email in config.");
        std::process::exit(1);
    }

    let author_email_str = author_email.as_ref().unwrap();

    // Determine timespan
    let timespan = if let Some(days) = cli.days {
        Timespan::days_back(days)
    } else {
        Timespan::days_back(config.default_timespan_days)
    };

    println!("Scanning: {}", scan_path.display());
    println!("Author: {}", author_email_str);
    println!("Timespan: {} days back\n", config.default_timespan_days);

    // Create orchestrator
    let orchestrator = Orchestrator::new(config)?;

    // Scan for repositories
    println!("🔍 Scanning for git repositories...");
    let repos = orchestrator.scan_repositories(&scan_path)?;

    if repos.is_empty() {
        println!("No git repositories found.");
        return Ok(());
    }

    println!("Found {} repositories\n", repos.len());

    // Analyze repositories
    println!("📊 Analyzing commits...");
    let results = orchestrator
        .analyze_repositories(&repos, Some(author_email_str.as_str()), &timespan)
        .await;

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
            println!("\nPlease edit the config file to add your Claude API key:");
            println!("  claude_api_key = \"sk-ant-YOUR_KEY_HERE\"");
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
