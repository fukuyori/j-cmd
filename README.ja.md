# j - 高速ディレクトリジャンプ

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/your-username/j-cmd)

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

`~/.zshrc` に追加：

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

設定を反映：
```zsh
source ~/.zshrc
```

### bash

`~/.bashrc` に追加：

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

設定を反映：
```bash
source ~/.bashrc
```

### PowerShell（Windows）

`$PROFILE` を編集して以下を追加：

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

設定後、PowerShell を再起動してください。

### コマンドプロンプト（Windows）

レジストリに自動実行を登録：

```cmd
reg add "HKCU\Software\Microsoft\Command Processor" /v AutoRun /t REG_SZ /d "C:\path\to\j-init.cmd" /f
```

## 使い方

### 基本操作

```bash
cd                  # ホームディレクトリへ
cd /var/log         # 通常のcd
cd proj             # "proj" を含むディレクトリへ（キーワード検索）
cd proj/src         # "proj" と "src" を含むパスへ
```

### 複数キーワード検索

スペース区切りで複数のキーワードを指定できます。キーワードは**順序通り**にマッチします。

```bash
cd first one        # "first" の後に "one" が来るパスへ
                    # 例: /home/first/project/one → ✅ マッチ
                    #     /home/one/project/first → ❌ 順序が違う
                    #     /home/second/project/one → ❌ "first" がない

cd work api src     # work → api → src の順で含むパスへ
                    # 例: /home/work/myapp/api/src → ✅ マッチ
```

### ドライブ指定（Windows のみ）

```cmd
cd d:               # D: ドライブのルートへ
cd d:proj           # D: ドライブ内の "proj" を検索
```

### 履歴操作

| コマンド | 説明 |
|---------|------|
| `cd -` | 前のディレクトリに戻る（Undo） |
| `cd +` | 次のディレクトリへ進む（Redo） |
| `cd .` | 最後に訪問したディレクトリへ |
| `cd -c` | 現在のディレクトリを履歴に記録 |
| `cd -x` | 現在のディレクトリを履歴から削除 |
| `cd -xa` | 履歴を全て消去 |
| `cd -l` | 履歴一覧を表示（デフォルト20件） |
| `cd -l 10` | 履歴を10件表示 |
| `cd -1` | 履歴の1番目（最新）に移動 |
| `cd -5` | 履歴の5番目に移動 |

### エイリアス

| コマンド | 説明 |
|---------|------|
| `cd -a doc` | 現在のディレクトリに "doc" エイリアスを登録 |
| `cd !doc` | エイリアス "doc" に移動 |
| `cd -ar doc` | エイリアス "doc" を削除 |
| `cd -al` | エイリアス一覧を表示 |

### インタラクティブモード（fzf連携）

fzf をインストールすると、インタラクティブに候補を選択できます。

```bash
cd -i               # 全履歴からfzfで選択
cd -i proj          # "proj" でフィルタしてfzfで選択
ji                  # cd -i の短縮形（zsh）
ji proj             # cd -i proj の短縮形
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
cd myproject

# エイリアス登録
cd ~/Documents
cd -a doc

# エイリアスで移動
cd !doc

# Undo/Redo
cd src           # /work/project/src へ移動
cd -             # 元のディレクトリへ戻る
cd +             # /work/project/src へ再び移動

# 履歴から選択
cd -l            # 履歴一覧を表示
cd -3            # 3番目の履歴へ移動
```

## マッチングルール

1. 現在のディレクトリからの相対パスを確認
2. 履歴から完全一致（ディレクトリ名）
3. 履歴から部分一致

- 大文字小文字は区別しない
- 最後のキーワードは最終ディレクトリ名と一致する必要がある
- **複数キーワードは指定した順序でマッチ**

例：
- `cd rust` は `/work/rust` にマッチするが `/work/rust/src` にはマッチしない
- `cd work rust` は `/home/work/project/rust` にマッチするが `/home/rust/project/work` にはマッチしない

## 設定ファイル

設定は `~/.config/j/` に保存されます：

```
~/.config/j/
├── state.json      # 履歴、undo/redo スタック
├── aliases.json    # エイリアス
└── config.json     # 除外パターン等の設定
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
