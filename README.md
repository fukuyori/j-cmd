# j - Fast Directory Jump

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-2.2.1-blue.svg)](https://github.com/your-username/j-cmd)

A fast directory jump command for Windows, Linux, and macOS.

**Jump by keyword, not by path.**

[日本語](README.ja.md)

## Features

- 🚀 Jump to directories by keyword
- 🔍 **Interactive selection with fzf**
- 📂 Browse directories with the built-in `j -d` selector
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

`j init` generates integration for the installed binary. By default it adds the `j` command without replacing the shell's standard `cd`.

```zsh
autoload -Uz compinit
compinit
eval "$(command j init zsh)"

# Use this instead only when you want j-cmd to extend cd
# eval "$(command j init zsh --cmd cd)"
```

Using `command j` invokes the `j` executable directly instead of an alias or function with the same name. This is especially important when reloading `.zshrc` while a `j()` function from the legacy manual setup is still defined in the current shell.

You can also source `j-init.zsh` from the repository. This file preserves the legacy behavior of extending `cd`.

```zsh
source /path/to/j-cmd/j-init.zsh
```

For manual configuration, add the following to `~/.zshrc`:

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
            result=$($J_CMD -)
            [[ -d "$result" ]] && builtin cd "$result"
            return
            ;;
        +)
            result=$($J_CMD +)
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
        .)
            result=$($J_CMD .)
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
        -c|-x|-xa|-a|-ar|-al|-h|--help|-V|--version|init|--ranking|--fzf-opts|--fzf-opts-clear)
            $J_CMD "$@"
            return
            ;;
        -d|--directory|-l|--list)
            result=$($J_CMD "$@")
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
        \!*)
            result=$($J_CMD "$arg")
            [[ -d "$result" ]] && builtin cd "$result" || echo "$result"
            return
            ;;
    esac

    if builtin cd "$@" 2>/dev/null; then
        return
    fi

    result=$($J_CMD "$@")
    [[ -d "$result" ]] && builtin cd "$result" || builtin cd "$@"
}

j() { cd "$@"; }
```

Apply changes:
```zsh
source ~/.zshrc
```

### bash

Add generated integration to `~/.bashrc`:

```bash
eval "$(j init bash)"

# Use this instead only when you want j-cmd to extend cd
# eval "$(j init bash --cmd cd)"
```

The repository setup file can be loaded with `source /path/to/j-cmd/j-init.sh`.

For manual configuration:

```bash
j() {
    local result
    result=$(/usr/local/bin/j "$@")
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

Add generated integration to your `$PROFILE`:

```powershell
Invoke-Expression (& j.exe init powershell | Out-String)

# Use this instead only when you want j-cmd to extend cd
# Invoke-Expression (& j.exe init powershell --cmd cd | Out-String)
```

The repository setup file can be loaded with `. C:\path\to\j-cmd\j-init.ps1`.

For manual configuration:

```powershell
function j {
    $prevOutputEncoding = [Console]::OutputEncoding
    $prevInputEncoding = [Console]::InputEncoding
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    [Console]::InputEncoding = [System.Text.Encoding]::UTF8
    try {
        if ($args.Count -eq 0) {
            $result = & j.exe
        } else {
            $result = & j.exe @args
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

### When Tab completion stops working

Replacing `cd` with a function, or loading fzf and other shell plugins, can overwrite completion definitions or the Tab key binding. As a rule, initialize `compinit` or bash-completion and fzf first, then load the j-cmd completion last.

#### zsh

Copying only the shell functions into `.zshrc` does not register the j-cmd completion function. To enable Tab completion, load the generated integration in this order:

```zsh
# 1. Initialize the zsh completion system
autoload -Uz compinit
compinit

# 2. Initialize fzf and other plugins here
# source ~/.fzf.zsh

# 3. Load j-cmd last (without replacing the standard cd)
eval "$(command j init zsh)"
```

If another plugin has already overwritten completion for `cd` or `j`, re-register it at the end of `.zshrc`:

```zsh
compdef _j_generated_complete j
# Only when using --cmd cd:
# compdef _j_generated_complete cd
```

To use j-cmd's jump behavior through `cd` while keeping zsh's standard directory path completion on Tab, add the following at the end of `.zshrc`, after `compinit` (or after loading Oh My Zsh):

```zsh
eval "$(command j init zsh --cmd cd)"
compdef _cd cd
```

Keep `compdef _cd cd` after the `j init` integration. This setup can also restore path completion when Tab completion for `cd` does not work. Tab completion for `j` still uses j-cmd's custom completer. Run `source ~/.zshrc` or open a new terminal to apply the change.

Check the current definitions with:

```zsh
type cd
print -r -- "${_comps[cd]}"
bindkey '^I'
```

When using `--cmd cd`, `_comps[cd]` should be `_j_generated_complete` for j-cmd's custom completion, or `_cd` for the standard path completion shown above. If it differs from your intended setting, fix the load order or run `compdef` again. Only when Tab is reported as `undefined-key`, restore standard completion with `bindkey '^I' expand-or-complete`. When using fzf-tab or another intentional Tab widget, leave the key binding unchanged and only re-register `compdef`.

If stale completion definitions remain, remove `.zcompdump`, restart the shell, and load `compinit` and the generated `j init zsh` integration again.

#### bash

Load bash-completion and fzf first, then load the j-cmd completion file last:

```bash
# Initialize your distribution's bash-completion and fzf first
# source /usr/share/bash-completion/bash_completion
# source ~/.fzf.bash

eval "$(j init bash)"

# To load only completion without generated integration:
# source /path/to/j-cmd/completions/j.bash
```

If another configuration overwrites the completion, re-register it at the end of `.bashrc`:

```bash
complete -F _j_generated_complete j
# Only when using --cmd cd:
# complete -F _j_generated_complete cd
complete -p j
```

#### Using fzf

`j -i` only requires the `fzf` executable to be available on PATH. fzf's shell key bindings and completion scripts are optional. If fzf shell integration breaks normal Tab completion, omit that integration and continue using `j -i`, or re-register the j-cmd completion after initializing fzf. The built-in `j -d` directory browser and `j -l` history selector do not use fzf.

#### PowerShell

On PowerShell, use the default `j init powershell` integration without replacing `cd`. It registers completion only for `j` and `ji`, preserving normal `cd` / `Set-Location` path completion. After changing your profile, start a new PowerShell session or reload it with:

```powershell
. $PROFILE
```

## Usage

### Basic Navigation

```bash
j                  # Jump to home directory
cd /var/log         # Use regular cd for an explicit path
j proj              # Jump to directory containing "proj" (keyword search)
j proj/src          # Jump to path containing "proj" and "src"
```

### Multiple Keyword Search

You can specify multiple keywords separated by spaces. Keywords are matched **in order**.

```bash
j first one         # Jump to path where "first" comes before "one"
                    # Example: /home/first/project/one → ✅ Match
                    #          /home/one/project/first → ❌ Wrong order
                    #          /home/second/project/one → ❌ No "first"

j work api src      # Jump to path containing work → api → src in order
                    # Example: /home/work/myapp/api/src → ✅ Match
```

### Drive Navigation (Windows only)

```cmd
j d:                # Jump to D: drive root
j d:proj            # Search "proj" in D: drive
```

### Directory Browser

Use `j -d` (or `j --directory`) to open the built-in directory browser. Each screen lists only the directories immediately below the displayed path. It does not use fzf or history to generate candidates.

```bash
j -d
```

Use `↑` / `↓` to select an entry, `→` to open the selected directory in the browser, and `←` to return to its parent. Typing characters searches directory names by a case-insensitive prefix and moves the highlight; `Backspace` removes the last search character. `Enter` confirms the highlighted directory as the jump destination without opening it in the browser; if the current directory has no child directories, it confirms the current directory. Press `.` to confirm the current directory immediately even when a child is highlighted. `Esc` or `Ctrl+C` cancels. The confirmed absolute path is passed back to the shell integration and recorded in history.

### History Operations

| Command | Description |
|---------|-------------|
| `j -` | Go back (Undo) |
| `j +` | Go forward (Redo) |
| `j .` | Jump to last visited directory |
| `j -c` | Record current directory to history |
| `j -x` | Remove current directory from history |
| `j -xa` | Clear all history |
| `j -l` | Select from the 20 most recent history entries |
| `j -l proj` | Select from 20 recent entries whose final directory name contains `proj` |

In the history selector, use `↑` / `↓` to move, `Enter` to jump, and `x` to delete the selected entry and refresh the list. Press `Esc` or `q` to cancel.

Missing history entries are not removed immediately. To protect temporarily disconnected network or external drives, j-cmd only prunes entries that are still missing more than 90 days after their last use.

### Aliases

| Command | Description |
|---------|-------------|
| `j -a doc` | Create alias "doc" for current directory |
| `j !doc` | Jump to alias "doc" |
| `j -ar doc` | Remove alias "doc" |
| `j -al` | List all aliases |

### Interactive Mode (fzf integration)

Install fzf to enable interactive selection from history.

When fzf is available, `j -i [keyword]` is recommended for normal history searches and selection. Use `j -l [keyword]` as a fallback when fzf is unavailable, to inspect the 20 most recent entries, or to delete missing and unwanted entries with the `x` key.

```bash
j -i                # Select from all history with fzf
j -i proj           # Filter by "proj" and select with fzf
ji                  # Shortcut for j -i
ji proj             # Shortcut for j -i proj
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
j myproject

# Create alias
cd ~/Documents
j -a doc

# Use alias
j !doc

# Undo/Redo
j src            # Jump to /work/project/src
j -              # Go back to previous directory
j +              # Go forward to /work/project/src again

# Select from history
j -l             # Select from the 20 most recent entries
j -l project     # Select from 20 recent entries containing "project"

# Select an immediate subdirectory
j -d
```

## Matching Rules

1. Check if path exists relative to current directory
2. Exact match on directory name in history
3. Partial match on directory name in history

- Case insensitive
- Last keyword must match the final directory name
- **Multiple keywords are matched in the specified order**

Examples:
- `j rust` matches `/work/rust` but not `/work/rust/src`
- `j work rust` matches `/home/work/project/rust` but not `/home/rust/project/work`

## Configuration Files

Configuration is stored in `~/.config/j/`:

```
~/.config/j/
├── state.json      # History and undo/redo stack
├── aliases.json    # Aliases
└── config.json     # Exclude patterns and settings
```

### Ranking

Configure the candidate order used by normal searches and `-i`. The default is `recent`, which prioritizes the most recently used entries.

```bash
j --ranking              # Show the current setting
j --ranking recent       # Prioritize last-used time (default)
j --ranking frecency     # Combine frequency and recency
j --ranking frequency    # Prioritize visit count
```

`-l` always lists the 20 most recent entries, regardless of the ranking setting.

### fzf Options

Pass additional options to fzf when using `-i`.

```bash
j --fzf-opts '--height 60% --reverse --border'
j --fzf-opts             # Show the current setting
j --fzf-opts-clear       # Remove the setting
```

When set, the `J_FZF_OPTS` environment variable takes precedence over `fzf_opts` in `config.json`.

```json
{
  "excludes": ["node_modules", ".git"],
  "ranking": "recent",
  "fzf_opts": "--height 60% --reverse --border"
}
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
