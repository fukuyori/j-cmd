# j - 高速ディレクトリジャンプ

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.9.0-blue.svg)](https://github.com/your-username/j-cmd)

Windows、Linux、macOS 対応の高速ディレクトリジャンプコマンド

**パスを覚えない。キーワードで移動。**

[English](README.md)

## 特徴

- 🚀 キーワードで素早くディレクトリ移動
- 📝 移動履歴の自動記録（最大1000件）
- ↩️ Undo/Redo対応
- 🏷️ エイリアス機能
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

`j` コマンドは移動先パスを出力するため、シェル関数で `cd` を実行する必要があります。

### bash（Linux / macOS）

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

### zsh（Linux / macOS）

`~/.zshrc` に追加：

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

設定を反映：
```zsh
source ~/.zshrc
```

### PowerShell（Windows）

`$PROFILE` を編集して以下を追加：

```powershell
# $PROFILE の場所を確認: echo $PROFILE
# 編集: notepad $PROFILE

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

## cd コマンドへのエイリアス（オプション）

`j` の代わりに `cd` を拡張したい場合の設定です。

### bash / zsh

`~/.bashrc` または `~/.zshrc` に追加：

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

`$PROFILE` に追加：

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

## 使い方

### 基本操作

```bash
j                   # ホームディレクトリへ
j src               # "src" を含むディレクトリへ
j proj/src          # "proj" と "src" を含むパスへ
j ~/work            # ~/work へ（チルダ展開）
j /usr/local        # 絶対パス
j ../sibling        # 相対パス
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
| `j -l` | 履歴一覧を表示（デフォルト20件） |
| `j -l 10` | 履歴を10件表示 |
| `j -1` | 履歴の1番目（最新）に移動 |
| `j -5` | 履歴の5番目に移動 |

### エイリアス

| コマンド | 説明 |
|---------|------|
| `j -a doc` | 現在のディレクトリに "doc" エイリアスを登録 |
| `j !doc` | エイリアス "doc" に移動 |
| `j -ar doc` | エイリアス "doc" を削除 |
| `j -al` | エイリアス一覧を表示 |

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
j src           # /work/project/src へ移動
j -             # 元のディレクトリへ戻る
j +             # /work/project/src へ再び移動

# 履歴から選択
j -l            # 履歴一覧を表示
j -3            # 3番目の履歴へ移動
```

## マッチングルール

1. 現在のディレクトリからの相対パスを確認
2. 履歴から完全一致（ディレクトリ名）
3. 履歴から部分一致

- 大文字小文字は区別しない
- 最後のキーワードは最終ディレクトリ名と一致する必要がある

例：`j rust` は `/work/rust` にマッチするが `/work/rust/src` にはマッチしない

## 設定ファイル

設定は `~/.config/j/` に保存されます：

```
~/.config/j/
├── state.json      # 履歴、undo/redo スタック
└── aliases.json    # エイリアス
```

## アンインストール

### Linux / macOS

```bash
# バイナリの削除
sudo rm /usr/local/bin/j

# 設定の削除
rm -rf ~/.config/j

# ~/.bashrc または ~/.zshrc から j 関数を削除
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
