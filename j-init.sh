# j-cmd shell wrapper for bash/zsh
# Add to ~/.bashrc or ~/.zshrc:
#   source /path/to/j-init.sh
# Or copy this function directly to your shell config

j() {
    local result
    result=$(/usr/local/bin/j "$@" 2>&1)
    if [ -d "$result" ]; then
        builtin cd "$result"
    elif [ -n "$result" ]; then
        echo "$result"
    fi
}
