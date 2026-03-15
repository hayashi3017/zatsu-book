# TODO

`design.md` を実装順に落とした作業バックログ。  
現状のリポジトリは最小の Rust crate だけなので、まずは workspace 化と mdBook の骨組み作成から始める。

## 実装原則

- [ ] 正本は `facts/*.yaml` とし、公開用 Markdown は `factctl` で生成する
- [ ] `src/` と `generated/` は生成物として扱い、差分が安定するよう常にソートする
- [ ] `src/` と `generated/` はコミットし、`book/` はコミットしない
- [ ] `SUMMARY.md` は手編集せず、自動生成前提で設計する
- [ ] MVP では mdBook preprocessor ではなく CLI 生成方式を採用する
- [ ] 初期公開は GitHub Pages の project site + GitHub Actions デプロイを前提にする
- [ ] `genres` と `tags` は内部 slug で持ち、表示名は taxonomy 定義から解決する

## Phase 0: 初期セットアップ

- [x] ルート `Cargo.toml` を workspace 構成へ移行する
- [x] `tools/factctl` crate を作成し、CLI のエントリポイントを移す
- [x] 既存のルート `src/main.rs` の扱いを決める
- [x] `book.toml` を追加する
- [x] `book.toml` の `site-url` を project site 前提で設定する
- [x] 以下の最小ディレクトリを作る
  - [x] `config/`
  - [x] `facts/`
  - [x] `generated/reports/`
  - [x] `generated/cache/`
  - [x] `schemas/`
  - [x] `templates/`
  - [x] `src/all/`
  - [x] `src/genres/`
  - [x] `src/tags/`
  - [x] `src/facts/`
  - [x] `src/updates/`
- [x] `config/taxonomy.yaml` の初版を作る
- [x] `templates/fact.yaml` の初版を作る
- [x] `templates/page.md.hbs` か `templates/page.md.j2` の初版を作る
- [x] `facts/` にサンプルデータを 1〜2 件追加し、end-to-end 検証用の最小入力を作る
- [x] `src/404.md` の扱いを決める

## Phase 1: データモデルとローダー

- [x] `Fact`, `Source`, `FactStatus` を定義する
- [x] `Taxonomy` モデルを定義する
- [x] `primary_genre` を `Fact` に追加する
- [x] `editorial`, `aliases`, `duplicate_of`, `supersedes`, `canonical`, `importance` を含む拡張フィールドを設計通りに持てるようにする
- [x] YAML ローダーを実装し、`facts/` を再帰走査できるようにする
- [x] taxonomy ローダーを実装する
- [x] ファイル名と `id` の一致チェックを実装する
- [x] `created_at` / `updated_at` を日付型または厳格な文字列として扱う
- [x] `duplicate_of` / `supersedes` の参照先解決に必要なインデックスを作る
- [x] テスト用 fixture を追加する

## Phase 2: validate 実装

- [x] `factctl validate` を実装する
- [x] 必須項目チェックを実装する
- [x] `status` enum チェックを実装する
- [x] `id` の命名規約チェックを実装する
- [x] `primary_genre` が `genres` に含まれることを検証する
- [x] `genres` / `tags` が taxonomy に定義済みであることを検証する
- [x] `sources` の最小件数チェックを実装する
- [x] URL 形式チェックを実装する
- [x] 日付形式チェックを実装する
- [x] `revision >= 1` を保証する
- [x] `duplicate_of` / `supersedes` の参照先存在チェックを実装する
- [x] `duplicate_of` / `supersedes` の循環参照チェックを実装する
- [x] `status=duplicate` のとき `duplicate_of` 必須など、状態と参照の整合チェックを実装する
- [x] `schemas/fact.schema.json` を追加し、CLI の検証と整合させる

## Phase 3: build-pages 実装

- [x] `factctl build-pages` を実装する
- [x] `published` のみ公開ページに含めるフィルタを実装する
- [x] 個別ページ生成を実装する
- [x] トップページ生成を実装する
- [x] 全件一覧生成を実装する
- [x] ジャンル一覧トップ生成を実装する
- [x] 各ジャンルページ生成を実装する
- [x] タグ一覧トップ生成を実装する
- [x] 各タグページ生成を実装する
- [x] 最近更新一覧生成を実装する
- [x] `src/SUMMARY.md` 自動生成を実装する
- [x] taxonomy の表示名を使ってジャンル名・タグ名を描画する
- [x] `src/404.md` を生成または配置する
- [x] 出力順を固定する
  - [x] ジャンル一覧は `updated_at desc`, 同点は `id asc`
  - [x] 更新一覧は `updated_at desc`
  - [x] 生成ディレクトリ・リンク順は常に安定化する
- [x] `generated/reports/unpublished.md` を出力する

## Phase 4: new / update 実装

- [x] `factctl new` を実装する
- [x] ジャンルごとの serial 採番を実装する
- [x] 4 桁 serial のシンプルな `id` / ファイル名を採用する
- [x] `primary_genre` をもとに保存先ディレクトリを決める
- [x] テンプレート YAML を新規作成できるようにする
- [x] `factctl update <id>` を実装する
- [x] `updated_at` 更新を実装する
- [x] `revision += 1` を実装する
- [x] エディタ起動の扱いを決める

## Phase 5: dedupe 実装

- [x] `factctl dedupe` を実装する
- [x] `claim` の正規化関数を実装する
- [x] `title`, `summary`, `claim`, `aliases` の比較前正規化を実装する
- [x] 完全重複チェックを実装する
  - [x] `id`
  - [x] 正規化済み `claim`
  - [x] 正規化済み主要ソース URL
- [x] 準重複チェックを実装する
- [x] 初期版は trigram Jaccard + title/claim 重み付き平均で実装する
- [x] ターミナル出力を整える
- [x] `generated/reports/duplicate_candidates.md` を出力する
- [x] 高信頼候補で fail できるオプションを実装する

## Phase 6: stale / doctor 実装

- [x] `factctl stale` を実装する
- [x] `accessed_at` が 180 日超のソースを抽出する
- [x] `kind != official` しかないレコードを抽出する
- [x] `generated/reports/stale_sources.md` を出力する
- [x] `factctl doctor` を実装する
- [x] `validate`, `dedupe`, `stale`, `build-pages --dry-run` 相当をまとめて実行する

## Phase 7: mdBook / CI / 運用整備

- [x] `mdbook build` が通る最小構成を整える
- [x] `Makefile` か `justfile` を追加する
- [x] `book/` を `.gitignore` に追加する
- [x] GitHub Actions で以下を自動化する
  - [x] format / lint
  - [x] `factctl validate`
  - [x] `factctl dedupe --fail-on-high-confidence-duplicate`
  - [x] `factctl build-pages`
  - [x] `git diff --exit-code -- src generated`
  - [x] `mdbook build`
- [x] GitHub Pages の `Source = GitHub Actions` 前提の公開設定を追加する
- [x] Pages deploy workflow を追加する
- [x] Pages workflow に `pages: write` と `id-token: write` を設定する
- [ ] project site の公開 URL 前提でリンク崩れがないことを確認する

## Phase 8: 品質強化

- [ ] ソース生存確認の自動化方針を決める
- [ ] official ドメイン許可リストの扱いを決める
- [ ] 関連ネタ推薦の設計を追加する
- [ ] alias / tag synonym を検索導線へ反映する
- [ ] テンプレート文面とトップページ導線を洗練する

## 受け入れ条件

- [x] `facts/*.yaml` から mdBook 用 Markdown を自動生成できる
- [x] `src/SUMMARY.md` を手編集しなくてよい
- [x] `factctl validate` で基本エラーを検出できる
- [x] `factctl dedupe` で重複候補を提示できる
- [x] `mdbook build` が成功する
- [x] `draft` が公開導線に出ない
- [x] `factctl update` で `revision` と `updated_at` を更新できる
- [ ] GitHub Pages の project site でアセットとリンクが崩れない

## 日々の実行コマンド

- [x] `cargo run -p factctl -- validate`
- [x] `cargo run -p factctl -- dedupe`
- [x] `cargo run -p factctl -- build-pages`
- [x] `mdbook build`

## 最初の一手

次に着手するなら以下の順で進める。

1. ルートを workspace 化し、`tools/factctl` を作る
2. `book.toml`、`config/taxonomy.yaml`、ディレクトリ骨組みを追加する
3. `Fact` モデル、`Taxonomy`、YAML ローダーを作る
4. `validate` を通して最初のサンプル fact と taxonomy を読み込めるようにする
5. `build-pages` で `src/`、`SUMMARY.md`、Pages 用導線を生成する
