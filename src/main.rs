mod cli;
mod config;
mod error;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use error::Result;

fn main() -> Result<()> {
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

    println!("dev-recap v{}", env!("CARGO_PKG_VERSION"));
    println!("AI-powered git commit summarizer for Demo Day presentations\n");
    println!("Config loaded from: {}", Config::default_config_path()?.display());
    println!("Author: {:?}", config.default_author_email);
    println!("Timespan: {} days", config.default_timespan_days);
    println!("Cache enabled: {}", config.cache_enabled);
    println!("\nReady to start implementation...");

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
                // TODO: Implement cache statistics
                println!("Cache statistics not yet implemented");
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
