# Examples

## Basic Usage

### Installing Packages

```bash
# Interactive: Browse all available packages with fuzzy search
pmgr install

# Direct: Install specific packages
pmgr install neovim ripgrep fd-find

# Quick install without confirmation (careful!)
pmgr install btop -y
```

### Removing Packages

```bash
# Interactive: Browse your installed packages and select ones to remove
pmgr remove

# Direct: Remove specific packages
pmgr remove firefox chromium

# Quick removal (be careful!)
pmgr remove bloatware -y
```

### Searching

```bash
# Search for packages
pmgr search browser
pmgr search "text editor"
pmgr search python
```

### Listing Installed Packages

```bash
# Simple text list
pmgr list

# Interactive browsing with preview
pmgr list -i
```

## Real-World Workflows

### Setting up a new development environment

```bash
# Install all your dev tools at once
pmgr install neovim git base-devel rust go nodejs npm docker

# Or use interactive mode to browse and pick
pmgr install
# Then search for "rust", "node", etc.
```

### Cleaning up unused packages

```bash
# Browse installed packages interactively
pmgr list -i
# Search for packages you recognize as unused

# Then remove them
pmgr remove
# Use fuzzy search to find and select multiple packages
```

### Finding alternatives

```bash
# Search for alternatives
pmgr search terminal
pmgr search file manager
pmgr search media player

# Install what you find interesting
pmgr install alacritty
```

## Comparison with Original `kif` Function

### Before (bash function)
```bash
function kif() {
  yay -Sl | fzf --multi --preview="yay -Si {1}/{2}" --preview-window=top,50% --bind 'enter:execute(yay -S {1}/{2})'
}

# Usage
kif  # Only installation
```

### Now (pmgr)
```bash
# Installation
pmgr install

# Removal
pmgr remove

# Listing
pmgr list -i

# Searching
pmgr search firefox

# Direct commands
pmgr i neovim
pmgr r bloatware
pmgr s browser
```

## Shell Integration

Add to your `~/.zshrc`:

```bash
# Replace your old kif function with:
alias kif='pmgr install'
alias kif-rm='pmgr remove'
alias kif-ls='pmgr list -i'

# Or use the built-in short aliases:
# pmgr i  -> install
# pmgr r  -> remove
# pmgr l  -> list
# pmgr s  -> search
```

## Pro Tips

1. **Multi-selection**: In interactive mode, use TAB to select multiple packages before confirming

2. **Preview**: The preview window shows full package information - very useful for checking dependencies

3. **Fuzzy search**: Type any part of the package name (e.g., "nvim" finds "neovim")

4. **Direct mode**: If you know what you want, skip interactive mode:
   ```bash
   pmgr i package1 package2 package3
   ```

5. **Combine with other tools**:
   ```bash
   # Get package list and process it
   pmgr list | grep python
   
   # Save package list for backup
   pmgr list > my-packages.txt
   ```
