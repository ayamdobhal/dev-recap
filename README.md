# dev-recap

> AI-powered git commit summarizer for Demo Day presentations

**dev-recap** is a CLI tool written in Rust that automatically analyzes git repositories, extracts commit history filtered by author and timespan, and generates AI-powered summaries perfect for Demo Day presentations.

## Features

- 🔍 **Recursive Repository Scanning** - Automatically finds all git repos in a directory
- 👤 **Author Filtering** - Filter commits by email with smart defaults from git config
- 📅 **Flexible Timespan** - Configure days back or custom date ranges
- 🤖 **AI-Powered Summaries** - Claude AI generates concise summaries with presentation tips
- 📊 **Progress Indicators** - Real-time progress bars during analysis
- 💾 **Smart Caching** - Avoid re-processing unchanged commits (TTL configurable)
- 🔗 **GitHub Integration** - Automatic PR link detection and formatting
- 💬 **Interactive Mode** - Smart prompts with defaults, press Enter to continue
- ⚙️ **Flexible Configuration** - Environment variables, config file, or CLI flags
- 🌐 **Custom API Endpoints** - Works with LiteLLM, custom proxies, and compatible APIs

## Installation

### Using Cargo (Recommended for System-Wide Install)

```bash
# Clone the repository
git clone https://github.com/yourusername/dev-recap.git
cd dev-recap

# Install system-wide (adds to ~/.cargo/bin)
cargo install --path .

# Or build release binary manually
cargo build --release
# Binary will be at: ./target/release/dev-recap

# Optional: Copy to a directory in your PATH
sudo cp target/release/dev-recap /usr/local/bin/
# Or for user-only:
cp target/release/dev-recap ~/.local/bin/
```

### Using Nix Flakes

```bash
# Clone the repository
git clone https://github.com/yourusername/dev-recap.git
cd dev-recap

# Enter development environment
nix develop

# Build release binary
cargo build --release

# Install system-wide
cargo install --path .
```

### Pre-built Binaries

Download the latest release from the [releases page](https://github.com/yourusername/dev-recap/releases) and place it in your PATH:

```bash
# Example for Linux/macOS
chmod +x dev-recap
sudo mv dev-recap /usr/local/bin/

# Or for user-only:
mv dev-recap ~/.local/bin/
```

### Verify Installation

After installation, verify it works:

```bash
# Check version
dev-recap --version

# View help
dev-recap --help

# Run in any directory
cd ~/projects
dev-recap
```

**Note:** If you installed with `cargo install`, make sure `~/.cargo/bin` is in your PATH:

```bash
# Add to ~/.bashrc, ~/.zshrc, or ~/.profile
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

### Interactive Mode

Simply run in any directory containing git repositories:

```bash
dev-recap
```

The tool will interactively prompt you for:
1. **Scan path** (defaults to current directory)
2. **Author email** (tries git config, then config file)
3. **Days back** (defaults from config or 14 days)

Press **Enter** to accept defaults, or type a new value.

### Non-Interactive Mode

Provide all options via CLI flags to skip prompts:

```bash
# Basic usage
dev-recap --author "you@example.com" --days 14

# Specify path
dev-recap --path ~/projects --author "you@example.com"

# Custom timespan
dev-recap --since "2025-01-01" --until "2025-01-15"

# Skip cache
dev-recap --no-cache
```

## Configuration

### Environment Variables (Highest Priority)

Compatible with Claude Code's environment variables:

```bash
export ANTHROPIC_AUTH_TOKEN="sk-ant-..."           # Your Claude API key
export ANTHROPIC_BASE_URL="http://localhost:4000" # Optional: LiteLLM or custom endpoint
```

### Config File

Create `~/.config/dev-recap/config.toml`:

```toml
# Default author email for filtering commits
default_author_email = "you@example.com"

# Claude API credentials (optional if using env vars)
claude_api_key = "sk-ant-..."                      # Or any auth token
claude_api_base_url = "https://api.anthropic.com" # Optional: custom base URL
claude_model = "claude-sonnet-4-5-20250929"       # Optional: model override

# Default timespan in days (2 weeks)
default_timespan_days = 14

# Directories/patterns to exclude from scanning
exclude_patterns = [
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "vendor",
]

# Maximum directory depth for scanning (optional)
max_scan_depth = 5

# Caching configuration
cache_enabled = true
cache_ttl_hours = 168  # 7 days

# GitHub token for enhanced rate limits (optional)
github_token = "ghp_..."
```

### Initialize Config

```bash
# Create default config file
dev-recap init

# View current config
dev-recap config

# Force overwrite existing config
dev-recap init --force
```

### Priority Order

1. **Environment Variables** (`ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`)
2. **Config File** (`~/.config/dev-recap/config.toml`)
3. **CLI Flags** (for per-run overrides)
4. **Defaults** (current directory, 14 days, git user.email)

## Commands

```bash
# Main command - analyze commits
dev-recap [OPTIONS]

# Initialize configuration
dev-recap init [--force]

# Show current configuration
dev-recap config

# Clear cache
dev-recap clear-cache

# Show cache statistics
dev-recap cache-stats
```

## CLI Options

```
OPTIONS:
    --path <PATH>              Path to scan for repositories [default: current dir]
    --author <EMAIL>           Author email to filter commits
    --days <DAYS>              Number of days to look back [default: 14]
    --since <DATE>             Start date (YYYY-MM-DD)
    --until <DATE>             End date (YYYY-MM-DD)
    --config <PATH>            Custom config file path
    --no-cache                 Disable caching for this run
    --max-depth <DEPTH>        Maximum directory depth to scan
    -h, --help                 Print help
    -V, --version              Print version
```

## Working with LiteLLM

dev-recap is fully compatible with LiteLLM and other Claude-compatible proxies:

```bash
# Start LiteLLM
litellm --model claude-3-5-sonnet-20241022

# Use with dev-recap (same env vars as Claude Code)
export ANTHROPIC_AUTH_TOKEN="your-litellm-key"
export ANTHROPIC_BASE_URL="http://localhost:4000"

dev-recap
```

The tool automatically appends `/v1/messages` to the base URL, matching Claude Code's behavior.

## How It Works

1. **Scan**: Recursively scans directories for git repositories
2. **Filter**: Extracts commits by author and timespan using libgit2
3. **Analyze**: Detects PR references (#123, GH-123), calculates statistics
4. **Summarize**: Sends structured prompt to Claude API for summary generation
5. **Cache**: Stores summaries keyed by repo path + commit hashes
6. **Present**: Displays results with repository info, stats, and AI insights

## Output Format

For each repository, dev-recap generates:

### Summary
A concise 2-3 paragraph overview of the work done during the timespan.

### Key Achievements
3-5 bullet points highlighting the most important contributions.

### Presentation Tips
3-5 practical tips for presenting this work in a demo or standup.

## Cache Management

```bash
# View cache statistics
dev-recap cache-stats

# Clear all cached summaries
dev-recap clear-cache

# Run without cache (doesn't update cache)
dev-recap --no-cache
```

Cache keys are based on repository path + commit hashes, so:
- ✅ New commits automatically invalidate cache
- ✅ Same commits = cache hit (no API call)
- ✅ Configurable TTL (default: 7 days)

## Development

### Prerequisites

- Nix with flakes enabled (or Rust 1.75+)
- libgit2
- OpenSSL (for HTTPS)

### Setup

```bash
# Enter development environment
nix develop

# Run in development mode
cargo run

# Run tests
cargo test

# Watch mode for development
cargo watch -x run

# Build release
cargo build --release
```

### Building Optimized Binaries

The project includes optimized release profile settings in `Cargo.toml`:

```bash
# Build fully optimized binary
cargo build --release

# The binary will be at: target/release/dev-recap
# Size: ~8-10MB (stripped)
# Optimizations: LTO enabled, single codegen unit, symbols stripped

# Test the release binary
./target/release/dev-recap --version

# Create distributable archive
tar -czf dev-recap-$(uname -s)-$(uname -m).tar.gz -C target/release dev-recap
```

**Cross-compilation examples:**

```bash
# For Linux (from macOS/Linux)
cargo build --release --target x86_64-unknown-linux-gnu

# For macOS (from macOS)
cargo build --release --target x86_64-apple-darwin    # Intel
cargo build --release --target aarch64-apple-darwin   # Apple Silicon

# For Windows (from any platform with mingw)
cargo build --release --target x86_64-pc-windows-gnu
```

### Project Structure

```
dev-recap/
├── src/
│   ├── main.rs           # Entry point, CLI handling, interactive prompts
│   ├── error.rs          # Error types using thiserror
│   ├── cli.rs            # CLI argument parsing with clap
│   ├── config.rs         # Configuration with env var priority
│   ├── orchestrator.rs   # Workflow coordination
│   ├── git/              # Git operations
│   │   ├── mod.rs        # Core types (Repository, Commit, etc)
│   │   ├── scanner.rs    # Recursive repo discovery
│   │   ├── parser.rs     # Commit extraction and filtering
│   │   ├── github.rs     # PR detection and GitHub URL parsing
│   │   └── stats.rs      # Statistics calculation
│   └── ai/               # AI integration
│       ├── mod.rs        # Summary type
│       ├── claude.rs     # Claude API client
│       ├── prompt.rs     # Prompt generation and response parsing
│       └── cache.rs      # Sled-based caching with TTL
├── Cargo.toml
├── flake.nix            # Nix development environment
└── README.md
```

## Use Cases

- 📢 **Demo Day Presentations** - Quickly recap your work for demos
- 📝 **Status Reports** - Generate work summaries for standups or reports
- 🎯 **Performance Reviews** - Comprehensive overview of contributions
- 🔄 **Sprint Retrospectives** - Review team progress across repositories

## Troubleshooting

### "No repositories found"
- Check that you're in a directory with git repositories
- Verify `exclude_patterns` isn't filtering out your repos
- Try increasing `max_scan_depth`

### "No commits found"
- Verify the author email matches git commit author
- Check the timespan includes commits
- Use `git log --author="email@example.com"` to verify commits exist

### API errors
- Verify `ANTHROPIC_AUTH_TOKEN` is set correctly
- Check `ANTHROPIC_BASE_URL` if using custom endpoint
- Ensure API key has sufficient credits

### "405 Method Not Allowed" with LiteLLM
- Ensure `ANTHROPIC_BASE_URL` is just the base (e.g., `http://localhost:4000`)
- Don't include `/v1/messages` - it's added automatically

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT

## Acknowledgments

Built with:
- [git2-rs](https://github.com/rust-lang/git2-rs) - Rust bindings for libgit2
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [sled](https://github.com/spacejam/sled) - Embedded database for caching
- [indicatif](https://github.com/console-rs/indicatif) - Progress indicators
- [Anthropic Claude](https://www.anthropic.com/) - AI summarization

---

**Made with 🦀 Rust**
