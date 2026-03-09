# j-cmd zsh integration
# ~/.zshrc に以下を追加:
#   source /path/to/j-init.zsh
#
# または、この内容を ~/.zshrc に直接コピー
#
# 注意: Tab補完を使用するには、このファイルを source する前に
# 以下が実行されている必要があります:
#   autoload -Uz compinit && compinit

# j バイナリのパス（環境に合わせて変更）
J_CMD="/usr/local/bin/j"

# ディレクトリ移動時に自動で履歴登録
chpwd() {
    $J_CMD -c 2>/dev/null
}

# cd を j で拡張
cd() {
    local result

    # 引数なし → ホームへ
    if [[ $# -eq 0 ]]; then
        builtin cd
        return
    fi

    local arg="$1"

    # j のオプションをそのまま渡す
    case "$arg" in
        # Undo/Redo
        -)
            result=$($J_CMD - 2>&1)
            [[ -d "$result" ]] && builtin cd "$result"
            return
            ;;
        +)
            result=$($J_CMD + 2>&1)
            if [[ -d "$result" ]]; then
                builtin cd "$result"
            else
                echo "$result"
            fi
            return
            ;;
        # 最後に訪問したディレクトリ
        .)
            result=$($J_CMD . 2>&1)
            if [[ -d "$result" ]]; then
                builtin cd "$result"
            else
                echo "$result"
            fi
            return
            ;;
        # インタラクティブモード
        -i|--interactive)
            shift
            result=$($J_CMD -i "$@" 2>&1)
            if [[ -d "$result" ]]; then
                builtin cd "$result"
            elif [[ -n "$result" ]]; then
                echo "$result"
            fi
            return
            ;;
        # 履歴操作
        -c|-x|-xa)
            $J_CMD "$@"
            return
            ;;
        # 履歴一覧
        -l|--list)
            $J_CMD "$@"
            return
            ;;
        # 履歴番号で移動 (-1, -2, -3, ...)
        -[0-9]|-[0-9][0-9]|-[0-9][0-9][0-9])
            result=$($J_CMD "$arg" 2>&1)
            if [[ -d "$result" ]]; then
                builtin cd "$result"
            else
                echo "$result"
            fi
            return
            ;;
        # エイリアス操作
        -a|-ar|-al)
            $J_CMD "$@"
            return
            ;;
        # 除外パターン
        --exclude-add|--exclude-remove|--exclude-list)
            $J_CMD "$@"
            return
            ;;
        # エイリアスで移動
        \!*)
            result=$($J_CMD "$arg" 2>&1)
            if [[ -d "$result" ]]; then
                builtin cd "$result"
            else
                echo "$result"
            fi
            return
            ;;
        # ヘルプ・バージョン
        -h|--help|-V|--version)
            $J_CMD "$arg"
            return
            ;;
    esac

    # 通常の cd を試す（成功したらそのまま終了）
    if builtin cd "$@" 2>/dev/null; then
        return
    fi

    # 失敗した場合、j でキーワード検索
    result=$($J_CMD "$@" 2>&1)
    if [[ -d "$result" ]]; then
        builtin cd "$result"
    else
        # j でも見つからない場合、元のエラーを表示
        builtin cd "$@"
    fi
}

# j コマンド（cd のエイリアス）
j() {
    cd "$@"
}

# ji: インタラクティブモードの短縮形
ji() {
    cd -i "$@"
}

# Tab補完設定
_j_complete() {
    local -a completions
    local keyword
    
    # 現在の入力を取得（2番目以降の単語）
    if [[ ${#words[@]} -gt 1 ]]; then
        keyword="${words[2,-1]}"
    fi
    
    # j --complete でマッチするパスを取得
    if completions=("${(@f)$($J_CMD --complete $keyword 2>/dev/null)}"); then
        # 空でない場合のみ補完候補として追加
        if [[ ${#completions[@]} -gt 0 && -n "${completions[1]}" ]]; then
            compadd -a completions
            return 0
        fi
    fi
    
    # フォールバック: 通常のディレクトリ補完
    _files -/
}

# 補完システムの初期化確認と登録
if (( $+functions[compdef] )); then
    compdef _j_complete j
    compdef _j_complete cd
fi
