# AGENTS.md

このリポジトリでは、`design.md` を上位設計として扱う。  
実装判断に迷ったら、まず `design.md` と `TODO.md` を確認する。

## プロジェクトの前提

- 目的は「へえーってなるネタ」を根拠付きで公開できる mdBook サイトを作ること
- mdBook は表示層であり、正本は `facts/*.yaml`
- `factctl` が validate / dedupe / build-pages / update / stale / doctor を担う
- 初期実装は custom preprocessor ではなく CLI 生成方式を優先する
- 初期公開は GitHub Pages の project site を前提にし、GitHub Actions で deploy する

## 正本と生成物

- `facts/` は正本データ。人が編集する
- `src/` は mdBook 入力だが、原則として `factctl build-pages` の生成物
- `generated/` は補助レポートや一時生成物の置き場
- `config/taxonomy.yaml` は genre / tag の表示名定義
- `src/SUMMARY.md` は必ず自動生成する
- `src/README.md` や `src/genres/**/README.md` も生成対象として扱う
- `src/` と `generated/` はコミット対象、`book/` は deploy 成果物なのでコミットしない

生成物を直接手で整えるより、テンプレートか generator を直すことを優先する。

## 実装優先順位

実装は次の順に進める。

1. workspace / `tools/factctl` / `book.toml` の初期化
2. `Fact` モデル、YAML ローダー、ディレクトリ走査
3. `factctl validate`
4. `factctl build-pages`
5. `factctl dedupe`
6. `factctl update`, `factctl stale`, `factctl doctor`
7. CI と公開導線

MVP の完了条件は `TODO.md` の「受け入れ条件」を基準にする。

## データ設計ルール

- 1 ファイル = 1 ネタ = 1 主張
- ファイル名と `id` は一致させる
- `id` は `<genre-slug>-<serial>-<short-slug>` 形式を守る
- `primary_genre` を必須とし、保存先ディレクトリと `id` 先頭 slug の基準にする
- `genres` と `tags` は表示名ではなく内部 slug で保持する
- 表示名は `config/taxonomy.yaml` から解決する
- 公開済み `id` は原則変更しない
- `status != published` のレコードは通常の公開ページに出さない
- `duplicate_of` と `supersedes` は参照先存在と循環なしを検証する
- `sources` は最低 1 件、できれば 2 件以上
- `accessed_at` は必須
- 一次情報と official ソースを優先する

## 生成ルール

- すべての一覧・ナビゲーション・リンク順は安定化する
- ソート順は設計書に従い、迷ったら `updated_at desc` と `id asc` を優先する
- `draft`, `duplicate`, `superseded`, `archived` は公開導線に混ぜない
- `generated/reports/unpublished.md` には非公開レコードを出してよい
- ジャンル、タグ、更新一覧、全件一覧、個別ページを同じモデルから生成する
- taxonomy の label を描画に使い、slug をそのまま表示しない
- GitHub Pages の project site 前提なので `book.toml` の `site-url` を repo path に合わせる

## バリデーションと重複判定

- validate では必須項目、enum、URL、日付、参照整合性、命名規約をまず固める
- validate では `primary_genre in genres` と taxonomy 定義済みチェックも行う
- dedupe では破壊的な自動統合をしない
- 初期の準重複判定は trigram Jaccard + title/claim の重み付き平均で十分
- 正規化は保守的に行い、元の文面は保持する
- 完全重複の判定基準は `id`、正規化済み `claim`、正規化済み主要ソース URL

## ファイル追加・変更時の同期対象

スキーマやデータ項目を変えたら、少なくとも以下を一緒に見直す。

- `design.md`
- `TODO.md`
- `config/taxonomy.yaml`
- `schemas/fact.schema.json`
- `tools/factctl/src/model.rs`
- `tools/factctl/src/load.rs`
- `tools/factctl/src/validate.rs`
- `tools/factctl/src/render.rs`
- `templates/`
- fixture / テストデータ

## 実装上の注意

- `src/` を生成物として扱うなら、手書きページを混在させない
- 生成順の不安定さは CI ノイズになるため、必ずソートする
- URL 正規化は query string を安易に落とさない
- 日本語正規化は重複判定用途に限定し、表示文言には適用しない
- `duplicate_of` / `supersedes` の自動補正はしない
- まず CLI を通してから UI や導線を磨く
- Pages 公開では `gh-pages` ブランチ運用ではなく Actions artifact deploy を使う
- project site では `src/404.md` を用意し、リンク崩れを早めに確認する

## 推奨ディレクトリ対応

- `config/taxonomy.yaml`
- `facts/<genre-slug>/<id>.yaml`
- `src/facts/<genre-slug>/<id>.md`
- `src/genres/<genre-slug>/README.md`
- `src/tags/<tag-slug>/README.md`
- `generated/reports/*.md`

## 作業前後の確認

変更時は可能な範囲で次を確認する。

- `cargo fmt`
- `cargo test`
- `cargo run -p factctl -- validate`
- `cargo run -p factctl -- build-pages`
- `mdbook build`
- GitHub Pages の project site URL でリンク確認

まだ未実装のコマンドがある段階では、追加した機能に対応する最小のテストか fixture を残す。

## 非目標

MVP では次を先送りしてよい。

- mdBook preprocessor の導入
- 関連ネタ自動推薦
- ソースの完全自動クローリング
- 高度な UI ギミック

## 引き継ぎメモ

- 現在の repo は最小 Rust crate から開始しているため、最初の大きな作業は workspace への再編成になる
- 実装の進行管理は `TODO.md` のチェックボックスを更新して行う
- 設計を変える場合は、コードだけ先行させず `design.md` か `TODO.md` に差分理由を残す
