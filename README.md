# dev-recap

> AI-powered git commit summarizer for Demo Day presentations

**dev-recap** is a CLI tool written in Rust that automatically analyzes git repositories, extracts commit history filtered by author and timespan, and generates AI-powered summaries perfect for Demo Day presentations.

## Features

- 🔍 **Recursive Repository Scanning** - Automatically finds all git repos in a directory
- 👤 **Author Filtering** - Filter commits by email with persistent defaults
- 📅 **Flexible Timespan** - Presets (1 week, 2 weeks, 1 month) or custom date ranges
- 🤖 **AI-Powered Summaries** - Claude AI generates concise summaries with presentation tips
- 📊 **Statistics Dashboard** - View commit counts, files changed, and LOC metrics
- 💾 **Smart Caching** - Avoid re-processing unchanged commits
- 👥 **Team Mode** - Summarize work by multiple team members
- 🔗 **GitHub Integration** - Automatic PR link detection and formatting
- ✅ **Interactive Commit Selection** - Optional mode to review and select specific commits
- 📝 **Multiple Export Formats** - Interactive viewer and markdown export
- 🎨 **Beautiful TUI** - Intuitive terminal user interface

## Installation

### Using Nix Flakes (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/dev-recap.git
cd dev-recap

# Enter development environment
nix develop

# Build and install
nix build
```

### Using Cargo

```bash
cargo install --path .
```

## Quick Start

### Interactive Mode (TUI)

Simply run in any directory containing git repositories:

```bash
dev-recap
```

This launches the interactive TUI where you can:
1. Choose Quick Mode or Review Mode
2. Select timespan (default: 2 weeks)
3. Enter author email(s)
4. Select repositories to analyze
5. Optionally review and select specific commits
6. View AI-generated summaries

### Non-Interactive Mode

```bash
# Basic usage
dev-recap --author "you@example.com" --days 14 --output summary.md

# Dry run to see what would be analyzed
dev-recap --dry-run

# Team mode
dev-recap --team --authors "alice@example.com,bob@example.com"

# Custom timespan
dev-recap --since "2025-01-01" --until "2025-01-15"

# Skip cache
dev-recap --no-cache
```

## Configuration

dev-recap looks for a configuration file at `~/.config/dev-recap/config.toml`:

```toml
# Default author email for filtering commits
default_author_email = "you@example.com"

# Claude API key
claude_api_key = "sk-ant-..."

# Default timespan in days (2 weeks)
default_timespan_days = 14

# Directories/patterns to exclude from scanning
exclude_patterns = [
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
]

# Caching configuration
cache_enabled = true
cache_ttl_hours = 168  # 7 days
```

### First Run

On first run, dev-recap will prompt you to create a configuration file. You can also create it manually.

## Development

### Prerequisites

- Nix with flakes enabled
- direnv (optional, but recommended)

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/dev-recap.git
cd dev-recap

# Allow direnv (if using direnv)
direnv allow

# Or manually enter nix shell
nix develop

# Run in development mode
cargo run

# Run tests
cargo test

# Watch mode for development
cargo watch -x run
```

### Project Structure

```
dev-recap/
├── src/
│   ├── main.rs           # Entry point
│   ├── error.rs          # Error types
│   ├── cli.rs            # CLI argument parsing
│   ├── config.rs         # Configuration management
│   ├── git/              # Git operations
│   ├── ai/               # Claude API integration
│   ├── tui/              # Terminal UI
│   └── output/           # Output formatting
├── Cargo.toml
├── flake.nix
└── README.md
```

## How It Works

1. **Scan**: Recursively scans directories for git repositories
2. **Filter**: Extracts commits by author and timespan
3. **Analyze**: Detects PR references, calculates statistics
4. **Summarize**: Sends structured data to Claude AI for summary generation
5. **Cache**: Stores summaries to avoid redundant API calls
6. **Present**: Displays results in beautiful TUI or exports to markdown

## Use Cases

- 📢 **Demo Day Presentations** - Quickly recap your work for demos
- 📝 **Status Reports** - Generate work summaries for standups or reports
- 🎯 **Performance Reviews** - Comprehensive overview of contributions
- 👥 **Team Updates** - Summarize team progress across multiple repos

## Roadmap

- [x] Core git parsing and AI integration
- [x] Interactive TUI
- [x] Caching layer
- [x] Team mode
- [x] Interactive commit selection
- [ ] GitLab support
- [ ] HTML export
- [ ] Custom AI prompt templates
- [ ] Slack/Discord integration

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT

## Acknowledgments

Built with:
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [git2-rs](https://github.com/rust-lang/git2-rs) - Git library bindings
- [Anthropic Claude](https://www.anthropic.com/) - AI summarization

---

**Made with 🦀 Rust**
