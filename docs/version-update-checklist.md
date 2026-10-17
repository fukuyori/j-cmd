# バージョン更新チェックリスト

バージョンを更新するときは、次のファイルと表示を同期する。

- `Cargo.toml`の`package.version`
- `Cargo.lock`内のローカルパッケージ`j`のバージョン
- `README.md`のVersionバッジとリリース用タグの例
- `README.ja.md`のVersionバッジとリリース用タグの例
- `CHANGELOG.md`のリリース見出し、日付、変更内容

更新後はPowerShellから次を確認する。

1. `cargo test`
2. `cargo build`
3. `target\debug\j.exe --version`
4. `rg -n "旧バージョン|新バージョン" Cargo.toml Cargo.lock README.md README.ja.md CHANGELOG.md`
5. `git diff --check`

タグを使ってリリースするときは、次も確認する。

1. タグ名から先頭の`v`を除いた値が`Cargo.toml`のバージョンと一致している
2. `main`上のGitHub ActionsがWindows、macOS、Linuxですべて成功している
3. タグのプッシュ後、GitHub Releaseに3環境のアーカイブと`SHA256SUMS`が添付されている

リリースビルド、パッケージ、インストーラー、タグ、コミット、プッシュは、依頼で明示された場合だけ実行する。
