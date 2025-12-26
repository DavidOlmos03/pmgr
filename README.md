# pmgr - Package Manager TUI

Modern TUI (Text User Interface) package manager for Arch Linux. A professional, scalable CLI tool built in Rust that provides an interactive fuzzy-finding interface for managing packages with `yay` or `pacman`.

## Features

- **Interactive Mode**: Fuzzy-finding interface powered by `skim` (Rust implementation of fzf)
- **Multi-selection**: Select multiple packages at once with TAB
- **Live Preview**: See package information while browsing
- **Fast & Efficient**: Built in Rust with optimized performance
- **Multiple Commands**: Install, remove, search, and list packages
- **Aliases**: Short command aliases for faster workflow
- **Color Output**: Beautiful colored terminal output

## Installation

### Build from source

```bash
# Clone or navigate to the project directory
cd syu

# Build the release binary
cargo build --release

# Install to your PATH
sudo cp target/release/pmgr /usr/local/bin/

# Or create a symlink
sudo ln -s "$(pwd)/target/release/pmgr" /usr/local/bin/pmgr
```

### Alternative: Install with cargo

```bash
cargo install --path .
```

## Usage

### Install Packages

```bash
# Interactive mode (default) - browse all available packages
pmgr install

# Install specific packages directly
pmgr install firefox chromium

# Skip interactive confirmation
pmgr install firefox -y

# Short alias
pmgr i
```

### Remove Packages

```bash
# Interactive mode (default) - browse installed packages
pmgr remove

# Remove specific packages directly
pmgr remove firefox chromium

# Skip interactive confirmation
pmgr remove firefox -y

# Short alias
pmgr r
```

### Search Packages

```bash
# Search for packages matching a query
pmgr search firefox

# Short alias
pmgr s firefox
```

### List Installed Packages

```bash
# Simple list (text output)
pmgr list

# Interactive browsing mode with fuzzy search
pmgr list --interactive
pmgr list -i

# Short alias
pmgr l
pmgr l -i
```

## Commands Reference

| Command | Alias | Description |
|---------|-------|-------------|
| `install [packages...]` | `i` | Install packages (interactive if no packages specified) |
| `remove [packages...]` | `r` | Remove packages (interactive if no packages specified) |
| `search <query>` | `s` | Search for packages |
| `list` | `l` | List installed packages |

### Flags

- `-i, --interactive`: Enable interactive browsing mode (for `list`)
- `-y, --no-interactive`: Skip interactive mode (for `install` and `remove`)

## Interactive Mode Controls

When in interactive mode:

- **Type**: Fuzzy search through packages
- **Arrow Keys**: Navigate up/down
- **TAB**: Select/deselect multiple packages
- **ENTER**: Confirm selection
- **ESC**: Cancel/exit
- **Preview Window**: Shows package information on the right

## Project Structure

```
pmgr/
├── src/
│   ├── main.rs              # Entry point & CLI parsing
│   ├── commands/            # Command implementations
│   │   ├── mod.rs
│   │   ├── install.rs       # Install command
│   │   ├── remove.rs        # Remove command
│   │   ├── search.rs        # Search command
│   │   └── list.rs          # List command
│   ├── package/             # Package manager logic
│   │   └── mod.rs           # yay/pacman interaction
│   └── ui/                  # User interface
│       └── mod.rs           # Fuzzy finder (skim)
├── Cargo.toml               # Dependencies & config
└── README.md
```

## Architecture & Design

### Built with Modern Rust Tools

- **clap** - CLI argument parsing with derive macros
- **skim** - Fuzzy finder (Rust implementation of fzf)
- **anyhow** - Error handling
- **colored** - Terminal colors
- **serde** - Serialization support

### Design Principles

1. **Separation of Concerns**: Commands, package logic, and UI are separate modules
2. **Error Handling**: Proper Result types with anyhow for context
3. **Interactive by Default**: Safer UX with preview before actions
4. **Extensible**: Easy to add new commands or package managers
5. **Performance**: Release builds with LTO and optimizations

### How it Works

1. **Package Manager Abstraction**: The `PackageManager` struct wraps `yay`/`pacman` commands
2. **Command Pattern**: Each command is a separate module with `execute()` method
3. **Interactive UI**: `skim` provides the fuzzy-finding TUI
4. **CLI Parsing**: `clap` handles argument parsing and help generation

## Advanced Usage

### Integration with Shell

Add to your `.zshrc` or `.bashrc`:

```bash
# Quick aliases
alias pmi='pmgr install'
alias pmr='pmgr remove'
alias pms='pmgr search'
alias pml='pmgr list -i'
```

### Alternative to your current `kif` function

Replace:
```bash
function kif() {
  yay -Sl | fzf --multi --preview="yay -Si {1}/{2}" --preview-window=top,50% --bind 'enter:execute(yay -S {1}/{2})'
}
```

With:
```bash
alias kif='pmgr install'
```

## Building Professional CLI Tools

This project demonstrates professional CLI development practices:

### 1. Project Structure
- Modular architecture with clear separation
- Commands, business logic, and UI in separate modules

### 2. Modern Rust Ecosystem
- **clap**: Industry-standard CLI parsing
- **anyhow/thiserror**: Professional error handling
- **skim/ratatui**: TUI frameworks

### 3. User Experience
- Interactive mode for safer operations
- Preview before action
- Multi-selection support
- Colored, formatted output

### 4. Development Workflow
```bash
# Development
cargo run -- install firefox

# Testing
cargo test

# Release build (optimized)
cargo build --release

# Check for issues
cargo clippy
```

## Future Enhancements

Potential additions for even more functionality:

- [ ] Configuration file support (`~/.config/pmgr/config.toml`)
- [ ] Update/upgrade commands
- [ ] Package groups management
- [ ] AUR helper integration improvements
- [ ] History tracking
- [ ] Export/import package lists
- [ ] Parallel package operations
- [ ] Custom themes/colors
- [ ] Plugin system

## License

MIT

## Author

David

---

**Note**: This tool is designed for Arch Linux and requires `yay` or `pacman` to be installed.
