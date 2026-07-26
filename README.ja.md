# j - 高速ディレクトリジャンプ

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-2.1.0-blue.svg)](https://github.com/your-username/j-cmd)

Windows、Linux、macOS 対応の高速ディレクトリジャンプコマンド

**パスを覚えない。キーワードで移動。**

[English](README.md)

## 特徴

- 🚀 キーワードで素早くディレクトリ移動
- 🔍 **fzf連携のインタラクティブ選択**
- 📝 移動履歴の自動記録（最大1000件）
- ↩️ Undo/Redo対応
- 🏷️ エイリアス機能
- 🚫 **除外パターン設定**
- ⌨️ **Tab補完対応**
- 💾 ドライブを跨いだ検索（Windows）
- 🌍 クロスプラットフォーム対応

## インストール

### Ubuntu / Debian

```bash
# 1. Rust のインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. ビルドに必要なパッケージ
sudo apt update
sudo apt install -y build-essential

# 3. ソースの取得とビルド
git clone https://github.com/your-username/j-cmd.git
cd j-cmd
cargo build --release

# 4. バイナリのインストール
sudo cp target/release/j /usr/local/bin/
```

### macOS

```bash
# 1. Rust のインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. ソースの取得とビルド
git clone https://github.com/your-username/j-cmd.git
cd j-cmd
cargo build --release

# 3. バイナリのインストール
sudo cp target/release/j /usr/local/bin/
```

### Windows

1. [Rust](https://rustup.rs/) をインストール
2. ソースの取得とビルド：
   ```powershell
   git clone https://github.com/your-username/j-cmd.git
   cd j-cmd
   cargo build --release
   ```
3. `target\release\j.exe` を PATH の通ったフォルダにコピー

## シェル設定

### zsh（推奨）

`j init`で現在のバイナリに合った設定を生成できます。通常は`j`コマンドを追加し、標準の`cd`は置き換えません。

```zsh
autoload -Uz compinit
compinit
eval "$(command j init zsh)"

# cdもj-cmdで拡張する場合だけ、代わりにこちらを使用
# eval "$(command j init zsh --cmd cd)"
```

`command j`と記述することで、同名のエイリアスや関数を経由せず、`j`実行ファイルを直接呼び出します。特に、従来の手動設定で定義された`j()`関数が現在のシェルに残ったまま`.zshrc`を再読み込みする場合に必要です。

リポジトリ内の`j-init.zsh`を読み込む方法もあります。このファイルは従来どおり`cd`を拡張します。

```zsh
source /path/to/j-cmd/j-init.zsh
```

手動で設定する場合は、`~/.zshrc`に以下を追加します。

```zsh
# j バイナリのパス
J_CMD="/usr/local/bin/j"

# ディレクトリ移動時に自動で履歴登録
chpwd() {
    $J_CMD -c 2>/dev/null
}

# cd を j で拡張
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
        -l|--list)
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

設定を反映：
```zsh
source ~/.zshrc
```

### bash

`~/.bashrc`に生成設定を追加します。

```bash
eval "$(j init bash)"

# cdもj-cmdで拡張する場合だけ、代わりにこちらを使用
# eval "$(j init bash --cmd cd)"
```

リポジトリ内の設定ファイルは`source /path/to/j-cmd/j-init.sh`で読み込めます。

手動で設定する場合：

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

設定を反映：
```bash
source ~/.bashrc
```

### PowerShell（Windows）

`$PROFILE`に生成設定を追加します。

```powershell
Invoke-Expression (& j.exe init powershell | Out-String)

# cdもj-cmdで拡張する場合だけ、代わりにこちらを使用
# Invoke-Expression (& j.exe init powershell --cmd cd | Out-String)
```

リポジトリ内の設定ファイルは`. C:\path\to\j-cmd\j-init.ps1`で読み込めます。

手動で設定する場合：

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

設定後、PowerShell を再起動してください。

### コマンドプロンプト（Windows）

レジストリに自動実行を登録：

```cmd
reg add "HKCU\Software\Microsoft\Command Processor" /v AutoRun /t REG_SZ /d "C:\path\to\j-init.cmd" /f
```

### Tab補完が効かない場合

`cd`を関数へ置き換えた後や、fzf・シェルプラグインを読み込んだ後は、補完定義やTabキーの割り当てが上書きされることがあります。基本方針は、`compinit`やbash-completion、fzfなどを先に初期化し、j-cmdの補完を最後に読み込むことです。

#### zsh

関数だけを`.zshrc`へコピーした場合、j-cmdの補完関数は登録されません。Tab補完を使用する場合は、次の順序で生成設定を読み込む方法を推奨します。

```zsh
# 1. zshの補完システム
autoload -Uz compinit
compinit

# 2. fzfやその他のプラグインをここで初期化
# source ~/.fzf.zsh

# 3. j-cmdは最後に読み込む（標準のcdは置き換えない）
eval "$(command j init zsh)"
```

すでに`cd`やTab補完が別のプラグインに上書きされている場合は、`.zshrc`の末尾で再登録できます。

```zsh
compdef _j_generated_complete j
# --cmd cdを使用している場合だけ:
# compdef _j_generated_complete cd
```

現在の登録状態は次のコマンドで確認できます。

```zsh
type cd
print -r -- "${_comps[cd]}"
bindkey '^I'
```

`--cmd cd`を使用している場合、`_comps[cd]`が`_j_generated_complete`でなければ、読み込み順を修正するか`compdef`を再実行してください。Tabが`undefined-key`の場合だけ、`bindkey '^I' expand-or-complete`で標準補完を復元できます。fzf-tabなどが意図的に別のウィジェットを使用している場合は、Tab割り当てを変更せず、`compdef`だけを再登録してください。

設定を直しても古い補完定義が残る場合は、`.zcompdump`を削除してからシェルを再起動し、再度`compinit`と`j init zsh`の生成設定を読み込んでください。

#### bash

bash-completionとfzfなどを先に読み込み、j-cmdの補完ファイルを最後に読み込みます。

```bash
# ディストリビューションのbash-completionやfzfを先に初期化
# source /usr/share/bash-completion/bash_completion
# source ~/.fzf.bash

eval "$(j init bash)"

# 生成設定を使わず、補完だけを読み込む場合:
# source /path/to/j-cmd/completions/j.bash
```

別の設定に補完を上書きされた場合は、`.bashrc`の末尾で再登録できます。

```bash
complete -F _j_generated_complete j
# --cmd cdを使用している場合だけ:
# complete -F _j_generated_complete cd
complete -p j
```

#### fzfとの併用

`j -i`が必要とするのはPATH上の`fzf`実行ファイルだけです。fzfのシェル向けキーバインドや補完スクリプトは必須ではありません。fzfのシェル統合によって通常のTab補完が壊れる場合は、その統合を読み込まずに`j -i`を使用するか、fzfの初期化後にj-cmdの補完を再登録してください。`j -l`の履歴選択画面はfzfを使用しません。

#### PowerShell

PowerShellでは`j init powershell`の既定設定を使い、`cd`を置き換えないことを推奨します。既定の`cd`／`Set-Location`のパス補完を維持したまま、`j`と`ji`だけに補完を登録します。プロファイルを変更した後は、新しいPowerShellを起動するか、次のコマンドで再読み込みします。

```powershell
. $PROFILE
```

## 使い方

### 基本操作

```bash
j                  # ホームディレクトリへ
cd /var/log         # 明示的なパスは通常のcd
j proj              # "proj" を含むディレクトリへ（キーワード検索）
j proj/src          # "proj" と "src" を含むパスへ
```

### 複数キーワード検索

スペース区切りで複数のキーワードを指定できます。キーワードは**順序通り**にマッチします。

```bash
j first one         # "first" の後に "one" が来るパスへ
                    # 例: /home/first/project/one → ✅ マッチ
                    #     /home/one/project/first → ❌ 順序が違う
                    #     /home/second/project/one → ❌ "first" がない

j work api src      # work → api → src の順で含むパスへ
                    # 例: /home/work/myapp/api/src → ✅ マッチ
```

### ドライブ指定（Windows のみ）

```cmd
j d:                # D: ドライブのルートへ
j d:proj            # D: ドライブ内の "proj" を検索
```

### 履歴操作

| コマンド | 説明 |
|---------|------|
| `j -` | 前のディレクトリに戻る（Undo） |
| `j +` | 次のディレクトリへ進む（Redo） |
| `j .` | 最後に訪問したディレクトリへ |
| `j -c` | 現在のディレクトリを履歴に記録 |
| `j -x` | 現在のディレクトリを履歴から削除 |
| `j -xa` | 履歴を全て消去 |
| `j -l` | 直近20件の履歴から上下キーで選択して移動 |
| `j -l proj` | 最後のフォルダ名に `proj` を含む直近20件の履歴から選択 |

履歴選択画面では、`↑` / `↓` で選択、`Enter` で移動、`x` で選択中の履歴を削除して一覧を再表示します。`Esc` または `q` でキャンセルできます。

存在しない履歴はすぐには削除しません。ネットワークドライブや外付けドライブの一時的な切断を考慮し、最後の使用から90日を超えても存在しない履歴だけを起動時に自動整理します。

### エイリアス

| コマンド | 説明 |
|---------|------|
| `j -a doc` | 現在のディレクトリに "doc" エイリアスを登録 |
| `j !doc` | エイリアス "doc" に移動 |
| `j -ar doc` | エイリアス "doc" を削除 |
| `j -al` | エイリアス一覧を表示 |

### インタラクティブモード（fzf連携）

fzf をインストールすると、インタラクティブに候補を選択できます。

fzfが利用できる場合、通常の履歴検索・選択には`j -i [キーワード]`を推奨します。`j -l [キーワード]`は、fzfがない環境での選択、直近20件の確認、存在しない履歴や不要な履歴を`x`キーで削除する場合に使用してください。

```bash
j -i                # 全履歴からfzfで選択
j -i proj           # "proj" でフィルタしてfzfで選択
ji                  # j -i の短縮形
ji proj             # j -i proj の短縮形
```

fzf のインストール:
```bash
# Ubuntu/Debian
sudo apt install fzf

# macOS
brew install fzf

# Windows (scoop)
scoop install fzf
```

### 除外パターン

特定のディレクトリを履歴検索から除外できます。

```bash
j --exclude-add node_modules     # node_modules を除外
j --exclude-add .git             # .git を除外
j --exclude-add "*.tmp"          # *.tmp を除外（ワイルドカード）
j --exclude-list                 # 除外パターン一覧
j --exclude-remove node_modules  # 除外解除
```

### 使用例

```bash
# プロジェクトディレクトリへ移動
j myproject

# エイリアス登録
cd ~/Documents
j -a doc

# エイリアスで移動
j !doc

# Undo/Redo
j src            # /work/project/src へ移動
j -              # 元のディレクトリへ戻る
j +              # /work/project/src へ再び移動

# 履歴から選択
j -l             # 直近20件から選択
j -l project     # "project" を含む直近20件から選択
```

## マッチングルール

1. 現在のディレクトリからの相対パスを確認
2. 履歴から完全一致（ディレクトリ名）
3. 履歴から部分一致

- 大文字小文字は区別しない
- 最後のキーワードは最終ディレクトリ名と一致する必要がある
- **複数キーワードは指定した順序でマッチ**

例：
- `j rust` は `/work/rust` にマッチするが `/work/rust/src` にはマッチしない
- `j work rust` は `/home/work/project/rust` にマッチするが `/home/rust/project/work` にはマッチしない

## 設定ファイル

設定は `~/.config/j/` に保存されます：

```
~/.config/j/
├── state.json      # 履歴、undo/redo スタック
├── aliases.json    # エイリアス
└── config.json     # 除外パターン等の設定
```

### ランキング

通常の検索と`-i`の候補順を設定できます。既定値は、直近で使用した履歴を優先する`recent`です。

```bash
j --ranking              # 現在の設定を表示
j --ranking recent       # 最終使用日時を優先（既定）
j --ranking frecency     # 使用回数と新しさを組み合わせる
j --ranking frequency    # 使用回数を優先
```

`-l`はランキング設定に関係なく、常に新しいものから直近20件を表示します。

### fzfオプション

`-i`で起動するfzfへ追加オプションを渡せます。

```bash
j --fzf-opts '--height 60% --reverse --border'
j --fzf-opts             # 現在の設定を表示
j --fzf-opts-clear       # 設定を削除
```

環境変数`J_FZF_OPTS`が設定されている場合は、`config.json`の`fzf_opts`より優先されます。

```json
{
  "excludes": ["node_modules", ".git"],
  "ranking": "recent",
  "fzf_opts": "--height 60% --reverse --border"
}
```

## アンインストール

### Linux / macOS

```bash
# バイナリの削除
sudo rm /usr/local/bin/j

# 設定の削除
rm -rf ~/.config/j

# ~/.zshrc または ~/.bashrc から設定を削除
```

### Windows

```powershell
# 設定の削除
Remove-Item -Recurse -Force "$env:USERPROFILE\.config\j"

# $PROFILE から j 関数を削除
# PATH から j.exe を削除
```

コマンドプロンプトの場合：
```cmd
reg delete "HKCU\Software\Microsoft\Command Processor" /v AutoRun /f
```

## ライセンス

MIT License
