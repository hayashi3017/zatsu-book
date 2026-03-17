# zatsu-book

`facts/*.yaml` を正本にして、`factctl` で mdBook 用 Markdown と運用レポートを生成するリポジトリです。

## Local Commands

```bash
make validate
make dedupe
make build-pages
make sync-generated
make stale
make book
make serve
make doctor
```

`make book` は `src/` を生成してから `mdbook build` を実行します。`make serve` は live preview 用に `book-serve/` を使うので、起動中でも `make book` と競合しません。`book/` と `book-serve/` は deploy / preview の成果物なのでコミットしません。
`facts/` を触ったら commit 前に `make sync-generated` を実行して `src/` と `generated/` の更新も一緒に含めてください。テンプレート変更だけなら `make build-pages` で十分です。

## CI

`.github/workflows/ci.yml` では次を自動実行します。

- `factctl validate`
- `factctl dedupe --fail-on-high-confidence-duplicate`
- `factctl build-pages`
- `./scripts/check-generated.sh`
- `mdbook build`

## GitHub Pages

`.github/workflows/pages.yml` は `main` への push で `book/html/` を生成し、GitHub Pages に deploy します。

Repository Settings では `Pages > Source` を `GitHub Actions` に設定してください。project site の URL は `https://hayashi3017.github.io/zatsu-book/` です。

## Release

既存の tag-driven Rust release workflow は `.github/workflows/rust-release.yml` のまま維持しています。`rust-vX.Y.Z` 形式の tag で `xtask` バイナリを release します。
