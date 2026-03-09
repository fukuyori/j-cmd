# j - Fast Directory Jump

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/your-username/j-cmd)

A fast directory jump command for Windows, Linux, and macOS.

**Jump by keyword, not by path.**

[日本語](README.ja.md)

## Features

- 🚀 Jump to directories by keyword
- 🔍 **Interactive selection with fzf**
- 📝 Automatic history recording (up to 1000 entries)
- ↩️ Undo/Redo support
- 🏷️ Alias support
- 🚫 **Exclude patterns**
- ⌨️ **Tab completion**
- 💾 Cross-drive search (Windows)
- 🌍 Cross-platform support

## Installation

### Ubuntu / Debian

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Install build dependencies
sudo apt update
sudo apt install -y build-essential

# 3. Clone and build
git clone https://github.com/your-username/j-cmd.git
cd j-cmd
cargo build --release

# 4. Install binary
sudo cp target/release/j /usr/local/bin/
```

### macOS

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Clone and build
git clone https://github.com/your-username/j-cmd.git
cd j-cmd
cargo build --release

# 3. Install binary
sudo cp target/release/j /usr/local/bin/
```

### Windows

1. Install [Rust](https://rustup.rs/)
2. Clone and build:
   ```powershell
   git clone https://github.com/your-username/j-cmd.git
   cd j-cmd
   cargo build --release
   ```
3. Copy `target\release\j.exe` to a directory in your PATH

## Shell Configuration

### zsh (Recommended)

Add to `~/.zshrc`:

```zsh
# j binary path
J_CMD="/usr/local/bin/j"

# Auto-record history on directory change
chpwd() {
    $J_CMD -c 2>/dev/null
}

# Extend cd with j
cd() {
    local result

    if [[ $# -eq 0 ]]; then
        builtin cd
        return
    fi

    local arg="$1"

    case "$arg" in
        -)
            result=$($J_CMD - 2>&1)
            [[ -d "$result" ]] && builtin cd "$result"
            return
            ;;
        +)
            result=$($J_CMD + 2>&1)
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
        .)
            result=$($J_CMD . 2>&1)
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
        -c|-x|-xa|-l|--list|-a|-ar|-al|-h|--help|-V|--version)
            $J_CMD "$@"
            return
            ;;
        -[0-9]|-[0-9][0-9]|-[0-9][0-9][0-9])
            result=$($J_CMD "$arg" 2>&1)
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
        \!*)
            result=$($J_CMD "$arg" 2>&1)
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
    esac

    if builtin cd "$@" 2>/dev/null; then
        return
    fi

    result=$($J_CMD "$@" 2>&1)
    [[ -d "$result" ]] && builtin cd "$result" || builtin cd "$@"
}

j() { cd "$@"; }
```

Apply changes:
```zsh
source ~/.zshrc
```

### bash

Add to `~/.bashrc`:

```bash
j() {
    local result
    result=$(/usr/local/bin/j "$@" 2>&1)
    if [ -d "$result" ]; then
        builtin cd "$result"
    elif [ -n "$result" ]; then
        echo "$result"
    fi
}
```

Apply changes:
```bash
source ~/.bashrc
```

### PowerShell (Windows)

Add to your `$PROFILE`:

```powershell
function j {
    $prevOutputEncoding = [Console]::OutputEncoding
    $prevInputEncoding = [Console]::InputEncoding
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    [Console]::InputEncoding = [System.Text.Encoding]::UTF8
    try {
        if ($args.Count -eq 0) {
            $result = & j.exe 2>&1
        } else {
            $result = & j.exe @args 2>&1
        }
        
        if ($result -is [array]) {
            foreach ($line in $result) {
                Write-Host $line
            }
            return
        }
        
        $output = "$result".Trim()
        if ($output -and (Test-Path -LiteralPath $output -PathType Container -ErrorAction SilentlyContinue)) {
            Set-Location -LiteralPath $output
        } elseif ($output) {
            Write-Host $output -ForegroundColor Yellow
        }
    } finally {
        [Console]::OutputEncoding = $prevOutputEncoding
        [Console]::InputEncoding = $prevInputEncoding
    }
}
```

Restart PowerShell after saving.

### Command Prompt (Windows)

Register auto-run in registry:

```cmd
reg add "HKCU\Software\Microsoft\Command Processor" /v AutoRun /t REG_SZ /d "C:\path\to\j-init.cmd" /f
```

## Usage

### Basic Navigation

```bash
cd                  # Jump to home directory
cd /var/log         # Regular cd
cd proj             # Jump to directory containing "proj" (keyword search)
cd proj/src         # Jump to path containing "proj" and "src"
```

### Multiple Keyword Search

You can specify multiple keywords separated by spaces. Keywords are matched **in order**.

```bash
cd first one        # Jump to path where "first" comes before "one"
                    # Example: /home/first/project/one → ✅ Match
                    #          /home/one/project/first → ❌ Wrong order
                    #          /home/second/project/one → ❌ No "first"

cd work api src     # Jump to path containing work → api → src in order
                    # Example: /home/work/myapp/api/src → ✅ Match
```

### Drive Navigation (Windows only)

```cmd
cd d:               # Jump to D: drive root
cd d:proj           # Search "proj" in D: drive
```

### History Operations

| Command | Description |
|---------|-------------|
| `cd -` | Go back (Undo) |
| `cd +` | Go forward (Redo) |
| `cd .` | Jump to last visited directory |
| `cd -c` | Record current directory to history |
| `cd -x` | Remove current directory from history |
| `cd -xa` | Clear all history |
| `cd -l` | List history (default 20 entries) |
| `cd -l 10` | List 10 history entries |
| `cd -1` | Jump to 1st (most recent) history entry |
| `cd -5` | Jump to 5th history entry |

### Aliases

| Command | Description |
|---------|-------------|
| `cd -a doc` | Create alias "doc" for current directory |
| `cd !doc` | Jump to alias "doc" |
| `cd -ar doc` | Remove alias "doc" |
| `cd -al` | List all aliases |

### Interactive Mode (fzf integration)

Install fzf to enable interactive selection from history.

```bash
cd -i               # Select from all history with fzf
cd -i proj          # Filter by "proj" and select with fzf
ji                  # Shortcut for cd -i (zsh)
ji proj             # Shortcut for cd -i proj
```

Install fzf:
```bash
# Ubuntu/Debian
sudo apt install fzf

# macOS
brew install fzf

# Windows (scoop)
scoop install fzf
```

### Exclude Patterns

Exclude specific directories from history search.

```bash
j --exclude-add node_modules     # Exclude node_modules
j --exclude-add .git             # Exclude .git
j --exclude-add "*.tmp"          # Exclude *.tmp (wildcard)
j --exclude-list                 # List exclude patterns
j --exclude-remove node_modules  # Remove pattern
```

### Examples

```bash
# Jump to project directory
cd myproject

# Create alias
cd ~/Documents
cd -a doc

# Use alias
cd !doc

# Undo/Redo
cd src           # Jump to /work/project/src
cd -             # Go back to previous directory
cd +             # Go forward to /work/project/src again

# Select from history
cd -l            # List history
cd -3            # Jump to 3rd history entry
```

## Matching Rules

1. Check if path exists relative to current directory
2. Exact match on directory name in history
3. Partial match on directory name in history

- Case insensitive
- Last keyword must match the final directory name
- **Multiple keywords are matched in the specified order**

Examples:
- `cd rust` matches `/work/rust` but not `/work/rust/src`
- `cd work rust` matches `/home/work/project/rust` but not `/home/rust/project/work`

## Configuration Files

Configuration is stored in `~/.config/j/`:

```
~/.config/j/
├── state.json      # History and undo/redo stack
├── aliases.json    # Aliases
└── config.json     # Exclude patterns and settings
```

## Uninstall

### Linux / macOS

```bash
# Remove binary
sudo rm /usr/local/bin/j

# Remove configuration
rm -rf ~/.config/j

# Remove settings from ~/.zshrc or ~/.bashrc
```

### Windows

```powershell
# Remove configuration
Remove-Item -Recurse -Force "$env:USERPROFILE\.config\j"

# Remove j function from $PROFILE
# Remove j.exe from PATH
```

For Command Prompt:
```cmd
reg delete "HKCU\Software\Microsoft\Command Processor" /v AutoRun /f
```

## License

MIT License
