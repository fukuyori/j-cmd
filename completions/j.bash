# j-cmd bash completion
# Install: source this file in your .bashrc
# e.g., source /path/to/j.bash
# or copy to /etc/bash_completion.d/j

_j_completions() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Options
    local options="-i -c -x -xa -l -a -ar -al -h --help -V --version --exclude-add --exclude-remove --exclude-list --complete"
    
    # If current word starts with -, complete options
    if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "$options" -- "$cur"))
        return
    fi
    
    # If previous word is an option that needs argument, don't complete
    case "$prev" in
        -a|-ar|--exclude-add|--exclude-remove)
            return
            ;;
    esac
    
    # Get directory completions from j --complete
    local keyword="${COMP_WORDS[*]:1}"
    local completions
    completions=$(j --complete $keyword 2>/dev/null)
    
    if [[ -n "$completions" ]]; then
        COMPREPLY=($(compgen -W "$completions" -- "$cur"))
    fi
}

complete -F _j_completions j
complete -F _j_completions cd  # If using cd alias
