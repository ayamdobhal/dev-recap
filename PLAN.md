# dev-recap - Project Implementation Plan

## Overview

**dev-recap** is a CLI tool written in Rust that automatically analyzes git repositories, extracts commit history filtered by author and timespan, and generates AI-powered summaries for Demo Day presentations.

## Core Value Proposition

Transforms the pre-demo scramble of manually reviewing merged PRs into an automated, comprehensive summary with presentation tips - all within seconds.

---

## Features

### Core Features

1. **Repository Scanner**
   - Recursive directory traversal
   - Git repository detection
   - Configurable exclude patterns
   - Max depth configuration

2. **Git Log Analyzer**
   - Author filtering (by email)
   - Timespan filtering (default: 2 weeks)
   - Commit metadata extraction (hash, message, date, files)
   - GitHub PR number detection from commit messages
   - Statistics collection (commit count, files changed, LOC)

3. **Claude AI Integration**
   - Structured prompt generation from git data
   - Per-repository work summaries
   - Presentation tips for showcasing work
   - Error handling and retry logic

4. **Interactive TUI**
   - Timespan selector (presets: 1 week, 2 weeks, 1 month + custom)
   - Author email input with persistence
   - Repository selection (multi-select checkboxes)
   - Progress indicators
   - Results viewer
   - Statistics dashboard

5. **Output System**
   - Interactive terminal viewer
   - Markdown export
   - Grouped by repository
   - Formatted with commit stats and GitHub links

### Enhanced Features

6. **Configuration Management**
   - Config file at `~/.config/dev-recap/config.toml`
   - Stores: default email, API key, exclude patterns, default timespan
   - CLI override support

7. **Caching Layer**
   - Cache AI summaries to avoid re-processing unchanged commits
   - Cache key: repo path + commit range hash
   - Configurable cache TTL
   - Cache invalidation support

8. **Team Mode**
   - Summarize work by multiple team members
   - Individual and aggregate summaries
   - Useful for team lead presentations

9. **GitHub PR Link Detection**
   - Parse commit messages for PR references (#123, GH-123)
   - Parse merge commit messages
   - Generate clickable GitHub links
   - Support org/repo format detection from remote URLs

10. **Statistics Dashboard**
    - Total commits per repository
    - Files changed
    - Lines of code (added/removed)
    - Commit frequency timeline
    - Top contributors (in team mode)

11. **Dry Run Mode**
    - Preview what will be analyzed
    - Show repositories found
    - Display commit count without API calls
    - Verify configuration

12. **Interactive Commit Selection**
    - Optional mode selectable at TUI startup
    - Review commits before sending to AI
    - Checkbox interface to include/exclude specific commits
    - Helps filter out irrelevant or WIP commits
    - Can be toggled on/off per session

---

## Technical Stack

### Rust Crates

| Purpose | Crate | Justification |
|---------|-------|---------------|
| TUI Framework | `ratatui` | Modern, actively maintained, excellent widget library |
| Terminal Handling | `crossterm` | Cross-platform, works with ratatui |
| Git Operations | `git2` | Stable libgit2 bindings, mature API |
| HTTP Client | `reqwest` | Industry standard, async support |
| Async Runtime | `tokio` | Required for reqwest, battle-tested |
| CLI Parsing | `clap` v4 | Derive macros, excellent UX |
| Config/Serialization | `serde`, `toml` | Standard for Rust configs |
| Date/Time | `chrono` | Comprehensive date operations |
| Error Handling | `anyhow`, `thiserror` | Ergonomic error management |
| Caching | `sled` or `cacache` | Embedded key-value storage |
| Regex | `regex` | PR number detection |

### Nix Setup

- **Nix Flakes** for reproducible development environment
- **direnv** for automatic environment activation
- Rust toolchain via `fenix` or `rust-overlay`
- Development dependencies: `rust-analyzer`, `cargo-watch`

---

## Project Structure

```
dev-recap/
├── flake.nix              # Nix flake definition
├── flake.lock             # Locked dependencies
├── .envrc                 # direnv configuration
├── Cargo.toml             # Rust project manifest
├── Cargo.lock             # Locked Rust dependencies
├── .gitignore
├── README.md
├── PLAN.md                # This file
├── src/
│   ├── main.rs            # Entry point, orchestration
│   ├── cli.rs             # Clap argument parsing
│   ├── config.rs          # Configuration management
│   ├── error.rs           # Error types
│   ├── git/
│   │   ├── mod.rs
│   │   ├── scanner.rs     # Directory traversal & repo discovery
│   │   ├── parser.rs      # Git log parsing & filtering
│   │   ├── stats.rs       # Statistics calculation
│   │   └── github.rs      # PR link detection & URL parsing
│   ├── ai/
│   │   ├── mod.rs
│   │   ├── claude.rs      # Claude API client
│   │   ├── prompt.rs      # Prompt templates
│   │   └── cache.rs       # AI response caching
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── app.rs         # Application state machine
│   │   ├── ui.rs          # UI rendering logic
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── mode.rs          # Mode selection (enable commit selection)
│   │       ├── timespan.rs      # Timespan selector widget
│   │       ├── author.rs        # Author input widget
│   │       ├── repos.rs         # Repository selector widget
│   │       ├── commits.rs       # Commit selector widget (optional)
│   │       ├── progress.rs      # Progress indicator
│   │       ├── stats.rs         # Statistics dashboard
│   │       └── results.rs       # Results viewer
│   └── output/
│       ├── mod.rs
│       ├── formatter.rs   # Terminal output formatting
│       └── markdown.rs    # Markdown export
└── tests/
    ├── integration/
    └── fixtures/
```

---

## Data Structures

### Configuration
```rust
struct Config {
    default_author_email: Option<String>,
    claude_api_key: String,
    default_timespan_days: u32,          // default: 14
    exclude_patterns: Vec<String>,        // e.g., ["node_modules", "target"]
    max_scan_depth: Option<u32>,
    cache_enabled: bool,
    cache_ttl_hours: u32,
    github_token: Option<String>,         // for API rate limits
}
```

### Repository Info
```rust
struct Repository {
    path: PathBuf,
    name: String,
    remote_url: Option<String>,
    github_info: Option<GitHubRepo>,
    commits: Vec<Commit>,
    stats: RepoStats,
}

struct GitHubRepo {
    owner: String,
    repo: String,
}
```

### Commit Data
```rust
struct Commit {
    hash: String,
    short_hash: String,
    author: Author,
    timestamp: DateTime<Utc>,
    message: String,
    summary: String,                      // First line
    body: Option<String>,
    files_changed: Vec<String>,
    insertions: u32,
    deletions: u32,
    pr_numbers: Vec<u32>,
}
```

### Statistics
```rust
struct RepoStats {
    total_commits: u32,
    total_files_changed: u32,
    total_insertions: u32,
    total_deletions: u32,
    commit_frequency: HashMap<String, u32>,  // date -> count
    pr_count: u32,
}
```

### AI Summary
```rust
struct Summary {
    repository: String,
    work_summary: String,                 // Markdown formatted
    key_achievements: Vec<String>,
    presentation_tips: Vec<String>,
    generated_at: DateTime<Utc>,
}
```

---

## Implementation Phases

### Phase 1: Project Setup & Infrastructure (Day 1)
**Goal:** Working development environment

- [x] Initialize git repository
- [ ] Create Nix flake with Rust toolchain
- [ ] Set up direnv
- [ ] Initialize Cargo project
- [ ] Add core dependencies to Cargo.toml
- [ ] Set up basic project structure (directories)
- [ ] Create error types with thiserror
- [ ] Basic README with project description

**Deliverable:** `cargo build` succeeds, nix flake works

### Phase 2: Configuration System (Day 1-2)
**Goal:** Config file loading and CLI parsing

- [ ] Define Config struct with serde
- [ ] Implement config file loading from `~/.config/dev-recap/config.toml`
- [ ] CLI arguments with clap (override config)
- [ ] Config validation
- [ ] Default config generation
- [ ] Unit tests for config loading

**Deliverable:** Config loads from file, CLI args work

### Phase 3: Git Integration (Day 2-3)
**Goal:** Extract git data from repositories

- [ ] Directory scanner (recursive walk)
- [ ] Git repository detection
- [ ] Commit filtering by author
- [ ] Commit filtering by timespan
- [ ] Parse commit metadata
- [ ] Calculate statistics (files, LOC)
- [ ] GitHub remote URL parsing
- [ ] PR number detection from commit messages
- [ ] Unit tests with fixture repos

**Deliverable:** Can scan directories and extract commit data

### Phase 4: AI Integration (Day 3-4)
**Goal:** Generate summaries with Claude

- [ ] Claude API client with reqwest
- [ ] Prompt template design
- [ ] Format git data into structured prompt
- [ ] Parse AI responses
- [ ] Error handling and retries
- [ ] Rate limiting
- [ ] Integration tests (with mock responses)

**Deliverable:** Can generate summaries from commit data

### Phase 5: Caching System (Day 4)
**Goal:** Avoid redundant API calls

- [ ] Choose caching backend (sled/cacache)
- [ ] Cache key generation (hash of commits)
- [ ] Cache storage and retrieval
- [ ] TTL implementation
- [ ] Cache invalidation
- [ ] Cache statistics (hit/miss rate)

**Deliverable:** Summaries are cached and reused

### Phase 6: Basic CLI Mode (Day 4-5)
**Goal:** Working CLI without TUI

- [ ] CLI-only mode implementation
- [ ] Terminal output formatter
- [ ] Markdown export
- [ ] Progress bars (indicatif crate)
- [ ] Dry run mode
- [ ] Statistics display

**Deliverable:** Fully functional CLI tool

### Phase 7: TUI Implementation (Day 5-7)
**Goal:** Interactive interface

- [ ] TUI app state machine
- [ ] Mode selection screen (enable/disable commit selection)
- [ ] Timespan selector widget
- [ ] Author email input widget
- [ ] Repository selector (checkboxes)
- [ ] Commit selector widget (optional mode)
- [ ] Progress indicator
- [ ] Statistics dashboard view
- [ ] Results viewer with scrolling
- [ ] Keyboard navigation
- [ ] Help screen

**Deliverable:** Full TUI experience with optional commit selection mode

### Phase 8: Team Mode (Day 7)
**Goal:** Multi-author support

- [ ] Multiple author input
- [ ] Per-author commit grouping
- [ ] Aggregate statistics
- [ ] Team summary prompt
- [ ] Team mode UI

**Deliverable:** Can analyze multiple team members

### Phase 9: Polish & Testing (Day 8-9)
**Goal:** Production ready

- [ ] Comprehensive error messages
- [ ] Edge case handling
- [ ] Integration tests
- [ ] Performance optimization
- [ ] Documentation (README, --help)
- [ ] Demo video/GIF
- [ ] Release build configuration

**Deliverable:** Release candidate v0.1.0

### Phase 10: Future Enhancements (Post-launch)
- [ ] GitLab support
- [ ] Bitbucket support
- [ ] HTML export
- [ ] JSON export for automation
- [ ] Web service mode
- [ ] Slack/Discord integration
- [ ] Custom AI prompt templates
- [ ] Diff preview in TUI

---

## Key Technical Decisions

### 1. Why Rust?
- Performance for scanning large directory trees
- Excellent TUI libraries (ratatui)
- Strong type system prevents bugs
- Great CLI ecosystem
- Single binary distribution

### 2. Caching Strategy
- Cache AI summaries keyed by commit hash range
- Use `sled` for embedded database (no external dependencies)
- Cache invalidation: TTL-based (default 7 days)
- Cache location: `~/.cache/dev-recap/`

### 3. GitHub PR Detection
- Regex patterns: `#(\d+)`, `GH-(\d+)`, `PR#(\d+)`
- Parse merge commits: `Merge pull request #(\d+)`
- Extract org/repo from git remote URL
- Format: `https://github.com/{org}/{repo}/pull/{number}`

### 4. AI Prompt Design
```
You are helping a developer prepare for Demo Day presentation.

Repository: {repo_name}
Timespan: {start_date} to {end_date}
Author: {author_email}

Commits ({count}):
{commit_list}

Statistics:
- Files changed: {files_changed}
- Insertions: {insertions}
- Deletions: {deletions}

Please provide:
1. A concise summary of the work done (2-3 paragraphs)
2. Key achievements (bullet points)
3. Tips for presenting this work in a screenshare demo (3-5 tips)

Format the response in Markdown.
```

### 5. TUI Navigation
- `Tab/Shift+Tab`: Navigate between widgets
- `Enter`: Confirm/select
- `Space`: Toggle checkboxes
- `Esc`: Go back/cancel
- `q`: Quit
- `?`: Help screen

### 6. TUI Flow
The interactive mode follows this screen flow:

1. **Mode Selection Screen**
   - Choose between "Quick Mode" (auto-include all commits) and "Review Mode" (select commits)
   - Default: Quick Mode

2. **Timespan Selection**
   - Presets: 1 week, 2 weeks, 1 month
   - Custom date range option

3. **Author Selection**
   - Enter author email(s)
   - Load from config default if available
   - Multi-author support for team mode

4. **Repository Selection**
   - Display all discovered repositories
   - Multi-select with checkboxes
   - Show commit counts per repo

5. **Commit Selection** (only if Review Mode enabled)
   - Show all commits per repository
   - Multi-select to include/exclude
   - Quick filters: "Select All", "Deselect All"

6. **Processing**
   - Progress bar for AI summarization
   - Show which repository is being processed

7. **Results Viewer**
   - Scrollable summary view
   - Statistics dashboard
   - Export to markdown option

---

## Configuration File Example

`~/.config/dev-recap/config.toml`:
```toml
# Default author email for filtering commits
default_author_email = "ayam@example.com"

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
    "__pycache__",
    ".venv"
]

# Maximum directory depth for scanning (optional)
# max_scan_depth = 5

# Caching configuration
cache_enabled = true
cache_ttl_hours = 168  # 7 days

# GitHub token for API access (optional, increases rate limits)
# github_token = "ghp_..."
```

---

## CLI Usage Examples

### Basic usage (launches TUI)
```bash
dev-recap
```

### Non-interactive mode
```bash
dev-recap --author "ayam@example.com" --days 14 --output summary.md
```

### Dry run
```bash
dev-recap --dry-run
```

### Team mode
```bash
dev-recap --team --authors "alice@example.com,bob@example.com"
```

### Specific directory
```bash
dev-recap --path ~/projects
```

### Custom timespan
```bash
dev-recap --since "2025-01-01" --until "2025-01-15"
```

### Skip cache
```bash
dev-recap --no-cache
```

---

## Success Metrics

- **Performance**: Scan 10 repos in < 5 seconds
- **Accuracy**: Detect 100% of commits by author
- **UX**: TUI is intuitive, no documentation needed for basic use
- **Reliability**: Graceful handling of network errors, corrupted git repos
- **Adoption**: Actually used every Demo Day!

---

## Potential Challenges & Mitigations

| Challenge | Mitigation |
|-----------|------------|
| Large repos slow to parse | Implement parallel scanning, progress indicators |
| API rate limits | Caching layer, batch requests when possible |
| Complex TUI state management | Use state machine pattern, separate rendering from logic |
| Git repositories without remote | Graceful degradation, use local path as identifier |
| Merge commits clutter output | Smart filtering, group related commits |
| API key security | Store in config file with appropriate permissions, warn user |

---

## Next Steps

1. **Review this plan** - Any changes or additions?
2. **Initialize repository** - Create the project structure
3. **Set up Nix flake** - Get development environment working
4. **Start Phase 1** - Begin implementation

---

## Decisions Made

- [x] **Watch mode**: Not included in v0.1.0 - can be added later if needed
- [x] **Commit categories**: Not included - keeping it simple for first release
- [x] **Conventional commits**: Not included - AI can infer purpose from commit messages
- [x] **Interactive commit selection**: ✅ INCLUDED - Optional mode selectable at TUI startup

---

**Last Updated**: 2025-10-24
**Status**: Planning Phase → Ready for Implementation
**Target v0.1.0 Release**: ~2 weeks from start
