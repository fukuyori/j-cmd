# j - Fast Directory Jump

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.9.0-blue.svg)](https://github.com/your-username/j-cmd)

A fast directory jump command for Windows, Linux, and macOS.

**Jump by keyword, not by path.**

[日本語](README.ja.md)

## Features

- 🚀 Jump to directories by keyword
- 📝 Automatic history recording (up to 1000 entries)
- ↩️ Undo/Redo support
- 🏷️ Alias support
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

The `j` command outputs the target path. A shell function is required to actually change directories.

### bash (Linux / macOS)

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

### zsh (Linux / macOS)

Add to `~/.zshrc`:

```zsh
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
```zsh
source ~/.zshrc
```

### PowerShell (Windows)

Add to your `$PROFILE`:

```powershell
# Check $PROFILE location: echo $PROFILE
# Edit: notepad $PROFILE

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

## cd Command Alias (Optional)

If you prefer to extend the `cd` command instead of using `j`:

### bash / zsh

Add to `~/.bashrc` or `~/.zshrc`:

```bash
cd() {
    if [ $# -eq 0 ]; then
        builtin cd
    elif [ -d "$1" ]; then
        builtin cd "$@"
    else
        local result
        result=$(/usr/local/bin/j "$@" 2>&1)
        if [ -d "$result" ]; then
            builtin cd "$result"
        else
            builtin cd "$@"
        fi
    fi
}
```

### PowerShell

Add to `$PROFILE`:

```powershell
function cd {
    param([string]$path)
    
    if (-not $path) {
        Set-Location ~
        return
    }
    
    if (Test-Path -LiteralPath $path -PathType Container -ErrorAction SilentlyContinue) {
        Set-Location -LiteralPath $path
        return
    }
    
    $result = & j.exe $path 2>&1
    if ($result -is [array]) {
        foreach ($line in $result) { Write-Host $line }
        return
    }
    
    $output = "$result".Trim()
    if ($output -and (Test-Path -LiteralPath $output -PathType Container -ErrorAction SilentlyContinue)) {
        Set-Location -LiteralPath $output
    } else {
        Set-Location -LiteralPath $path
    }
}
```

## Usage

### Basic Navigation

```bash
j                   # Jump to home directory
j src               # Jump to directory containing "src"
j proj/src          # Jump to path containing "proj" and "src"
j ~/work            # Jump to ~/work (tilde expansion)
j /usr/local        # Absolute path
j ../sibling        # Relative path
```

### Drive Navigation (Windows only)

```cmd
j d:                # Jump to D: drive root
j d:proj            # Search "proj" in D: drive
```

### History Operations

| Command | Description |
|---------|-------------|
| `j -` | Go back (Undo) |
| `j +` | Go forward (Redo) |
| `j .` | Jump to last visited directory |
| `j -c` | Record current directory to history |
| `j -x` | Remove current directory from history |
| `j -xa` | Clear all history |
| `j -l` | List history (default 20 entries) |
| `j -l 10` | List 10 history entries |
| `j -1` | Jump to 1st (most recent) history entry |
| `j -5` | Jump to 5th history entry |

### Aliases

| Command | Description |
|---------|-------------|
| `j -a doc` | Create alias "doc" for current directory |
| `j !doc` | Jump to alias "doc" |
| `j -ar doc` | Remove alias "doc" |
| `j -al` | List all aliases |

### Examples

```bash
# Jump to project directory
j myproject

# Create alias
cd ~/Documents
j -a doc

# Use alias
j !doc

# Undo/Redo
j src           # Jump to /work/project/src
j -             # Go back to previous directory
j +             # Go forward to /work/project/src again

# Select from history
j -l            # List history
j -3            # Jump to 3rd history entry
```

## Matching Rules

1. Check if path exists relative to current directory
2. Exact match on directory name in history
3. Partial match on directory name in history

- Case insensitive
- Last keyword must match the final directory name

Example: `j rust` matches `/work/rust` but not `/work/rust/src`

## Configuration Files

Configuration is stored in `~/.config/j/`:

```
~/.config/j/
├── state.json      # History and undo/redo stack
└── aliases.json    # Aliases
```

## Uninstall

### Linux / macOS

```bash
# Remove binary
sudo rm /usr/local/bin/j

# Remove configuration
rm -rf ~/.config/j

# Remove j function from ~/.bashrc or ~/.zshrc
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
