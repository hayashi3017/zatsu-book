# design.md

## 1. 目的

「へえーってなるネタ」を、**根拠付き**かつ**ジャンル別**に公開できる mdBook サイトを構築する。  
運用では次を重視する。

- ネタの**追加が簡単**であること
- 既存ネタとの**重複を防げる**こと
- ソース差し替えや説明改善などの**更新が容易**であること
- 公開ページは mdBook による**一覧性・検索性**を持つこと
- Codex が段階的に実装しやすいよう、**データ構造・生成手順・CI** まで明示すること

---

## 2. 設計方針

### 2.1 mdBook は表示層に徹する

mdBook は以下の特性を持つ。

- `src/SUMMARY.md` で章構成を管理する
- `README.md` を `index.html` として扱う組み込み preprocessor がある
- custom preprocessor により、レンダリング前の Markdown を加工できる
- `mdbook build` により静的 HTML を生成できる
- HTML 出力は検索機能付きで公開しやすい

このため、**Markdown を手で正本管理するのではなく、ネタデータを正本として持ち、mdBook 向け Markdown を生成する**構成を採用する。

### 2.2 1ネタ = 1主張

最小管理単位は「へえー1個」にする。

例:

- 1円玉の木は特定の木ではない
- 500円玉の斜めギザは大量生産型貨幣として世界初
- くまモンは営業部長兼しあわせ部長で、いちおう公務員

この粒度にすることで、以下が容易になる。

- 重複判定
- 既存ネタの更新
- 複数ジャンルへの横断配置
- ソース差し替え

### 2.3 正本はデータファイルで持つ

運用の正本は `facts/` 以下の YAML ファイル群とする。  
個別 Markdown や一覧 Markdown はすべて生成物とする。

### 2.4 初期公開と内部表現の前提

初期公開は **GitHub Pages の project site** を前提にする。  
URL は `https://<owner>.github.io/<repo>/` 形式とし、初期段階では独自ドメインを使わない。

また、`genres` と `tags` は表示名ではなく **内部 slug** で保持する。  
表示名や説明は taxonomy 定義ファイルから解決する。

さらに、各 fact には保存パスと `id` の先頭 genre を決めるための `primary_genre` を持たせる。

---

## 3. アーキテクチャ概要

```text
[ facts/*.yaml ]
      |
      v
[ factctl validate / dedupe / build-pages ]
      |
      +--> [ generated metadata / reports ]
      |
      +--> [ src/SUMMARY.md ]
      +--> [ src/README.md ]
      +--> [ src/genres/** ]
      +--> [ src/facts/** ]
      +--> [ src/tags/** ]
      +--> [ src/updates/** ]
      |
      v
[ mdbook build ]
      |
      v
[ book/ ]  <- 公開用静的HTML
```

### 採用コンポーネント

- **mdBook**: 公開用静的サイト生成
- **Rust 製 CLI (`factctl`)**: データ検証、重複検知、ページ生成、運用支援
- **YAML Schema / バリデーション**: 入力品質担保
- **CI**: 自動検証とビルド

### optional

将来的に必要なら mdBook preprocessor を導入してもよいが、初期実装は **生成 CLI 方式** を優先する。  
理由は、以下の通り。

- ローカルでのデバッグが簡単
- 生成差分が Git 上で見やすい
- Codex による実装範囲を分割しやすい
- preprocessor より責務が明確

---

## 4. ディレクトリ構成

```text
repo/
├─ Cargo.toml
├─ Cargo.lock
├─ book.toml
├─ config/
│  └─ taxonomy.yaml
├─ facts/
│  ├─ money/
│  │  ├─ money-001-yen-tree-not-specific.yaml
│  │  └─ money-002-500yen-diagonal-reeding.yaml
│  ├─ mascots/
│  ├─ science/
│  ├─ food/
│  └─ local/
├─ generated/
│  ├─ reports/
│  │  ├─ duplicate_candidates.md
│  │  ├─ stale_sources.md
│  │  └─ unpublished.md
│  └─ cache/
├─ src/
│  ├─ SUMMARY.md
│  ├─ README.md
│  ├─ all/README.md
│  ├─ genres/
│  │  ├─ README.md
│  │  ├─ money/README.md
│  │  ├─ mascots/README.md
│  │  ├─ science/README.md
│  │  └─ food/README.md
│  ├─ tags/
│  │  ├─ README.md
│  │  └─ ...
│  ├─ facts/
│  │  ├─ money/
│  │  ├─ mascots/
│  │  └─ ...
│  └─ updates/
│     └─ README.md
├─ tools/
│  └─ factctl/
│     ├─ src/
│     └─ tests/
├─ schemas/
│  └─ fact.schema.json
├─ templates/
│  ├─ fact.yaml
│  └─ page.md.hbs
└─ .github/
   └─ workflows/
      ├─ ci.yml
      └─ pages.yml
```

---

## 5. データモデル

### 5.1 基本スキーマ

1ネタ1ファイル。ファイル名と `id` は一致させる。

```yaml
id: money-001-yen-tree-not-specific
title: 1円玉の木は特定の木ではない
primary_genre: money
genres:
  - money
  - japan
tags:
  - currency
  - coin-design
summary: 1円玉の表の木は特定の樹種ではなく、造幣局では若木として説明している。
claim: 1円玉のデザインの木には具体的な木の名前はなく、造幣局では若木と表現している。
explanation: |
  1円玉の木は特定の樹種が定められているように見えるが、造幣局では
  「具体的な木の名前はない」としており、若木と説明している。
  思い込みとのギャップがあるため、「へえー」となりやすいネタである。
sources:
  - id: mint-faq-coin
    url: https://www.mint.go.jp/faq-list/faq_coin
    title: 貨幣に関するよくある質問
    publisher: 造幣局
    kind: official
    accessed_at: 2026-03-14
    quoted_fact: 1円貨のデザインの木には具体的な木の名前はない
status: published
created_at: 2026-03-14
updated_at: 2026-03-14
revision: 1
aliases:
  - 一円玉の木は何の木？
  - 1円玉の木は何の木？
duplicate_of: null
supersedes: null
canonical: true
importance: 0.72
editorial:
  tone: casual
  audience: general
  spoiler: false
```

`genres` と `tags` は内部 slug とし、サイト表示時のラベルは `config/taxonomy.yaml` から解決する。

### 5.2 必須項目

- `id`
- `title`
- `primary_genre`
- `genres`
- `summary`
- `claim`
- `sources`
- `status`
- `created_at`
- `updated_at`
- `revision`

### 5.3 状態遷移

`status` は次を持つ。

- `draft`: 下書き。公開対象外
- `published`: 公開中
- `duplicate`: 重複として統合済み
- `superseded`: より新しい説明に置換済み
- `archived`: 公開終了。残すが通常導線には出さない

### 5.4 taxonomy 定義

ジャンルとタグの表示名は別ファイルで管理する。

例:

```yaml
genres:
  money:
    label: お金
  japan:
    label: 日本
tags:
  currency:
    label: 貨幣
  coin-design:
    label: デザイン
```

これにより、次を両立できる。

- データ内部表現は安定した ASCII slug で保つ
- 表示名だけを後から差し替えられる
- URL とページ生成規則を単純に保てる

### 5.5 参照関係

- `duplicate_of`: 同一主張の既存 ID を指す
- `supersedes`: 旧ネタを置き換えるときの参照
- `canonical`: 公開上の正本フラグ

---

## 6. ID と命名規約

### 6.1 ID 形式

```text
<genre-slug>-<serial4>
```

例:

- `money-0001`
- `local-0004`
- `weather-0123`

### 6.2 ルール

- 小文字英数字とハイフンのみ
- serial は 4 桁の連番を使う
- 一度公開した `id` は原則変更しない
- 同一主張を更新しても `id` は維持する
- 移行期間中は旧形式 `<genre-slug>-<serial>-<short-slug>` も読み込み対象として許容する
- 表示タイトルの変更は可能

---

## 7. 重複防止設計

重複は 3 段階で扱う。

### 7.1 完全重複チェック

次のいずれかが一致したらエラーにする。

- `id`
- 正規化済み `claim`

同じ主要ソース URL を参照していても、FAQ や索引ページを複数ネタで共有することがあるため、
URL 一致だけでは完全重複エラーにしない。共有 URL は準重複確認時の補助情報として扱う。

### 7.2 準重複チェック

類似度ベースで既存候補を警告表示する。

対象フィールド:

- `title`
- `summary`
- `claim`
- `aliases`

正規化手順:

- Unicode NFKC
- 全角半角統一
- 句読点除去
- 連続空白圧縮
- 数字表現の正規化
- ひらがな/カタカナ表記揺れの軽減（可能なら）

スコアリング候補:

- trigram Jaccard
- cosine similarity (bag of words)
- SimHash

初期実装では、**trigram Jaccard + title/claim の重み付き平均**で十分。

### 7.3 運用判定

重複候補が出た場合、運用者は次のいずれかを選ぶ。

- 新規として登録する
- 既存レコードを更新する
- `duplicate_of` を設定して統合する

### 7.4 判定ポリシー

#### 同一ネタとして扱う例

- 1円玉の木は何の木か決まっていない
- 1円玉の木には特定の樹種名がない

#### 別ネタでよい例

- 500円玉の斜めギザは世界初
- 新500円玉は3種類の金属を使っている

---

## 8. ソース管理ポリシー

### 8.1 優先順位

1. 官公庁・公的機関・一次情報
2. 公式団体・大学・研究機関
3. 補助的な二次情報

### 8.2 最低要件

- `sources` は最低 1 件必須
- 理想は 2 件以上
- `kind=official` を優先
- `accessed_at` を必須化

### 8.3 stale 判定

定期的にソース点検対象を洗い出すため、以下を stale 候補とする。

- アクセスから 180 日超
- 404 / リダイレクト不整合
- `kind != official` かつ補助ソースしかない

`factctl stale` でレポートを出す。

---

## 9. 公開ページ設計

### 9.1 必須ページ

- トップページ
- 全件一覧
- ジャンル一覧トップ
- 各ジャンルページ
- タグ一覧トップ
- 各タグページ
- 最近更新一覧
- 個別ネタページ

### 9.2 任意ページ

- 人気ネタ一覧
- ランダム表示導線
- ソースの見方
- このサイトについて
- 投稿・編集ガイド

### 9.3 トップページ案

```text
- このサイトについて
- 今日のおすすめ 3 件
- 新着
- 最近更新
- 人気ジャンル
- 初めて読む人向け
```

### 9.4 ジャンルページ案

```text
# お金
- 概要
- 件数
- 新着順一覧
- 名前順一覧
- 関連タグ
```

### 9.5 個別ページテンプレート

```md
# 1円玉の木は特定の木ではない

## 要点
1円玉の表の木は特定の樹種ではなく、造幣局では若木として説明している。

## 根拠
- 造幣局「貨幣に関するよくある質問」
- 種別: 公式
- 最終確認日: 2026-03-14

## 解説
1円玉の木は「何の木か決まっている」と思われがちだが、公式にはそうではない。

## ジャンル
- お金
- 日本

## タグ
- 貨幣
- デザイン
```

---

## 10. mdBook 連携方針

### 10.1 `SUMMARY.md` は自動生成

`SUMMARY.md` は厳密な構造を要求されるため、手編集ではなく生成対象にする。  
公開ページ追加・ジャンル追加・タグ追加に伴い、`factctl build-pages` が再生成する。

### 10.2 `README.md` を index として使う

ジャンルトップや一覧トップは `README.md` を使って index 化する。  
これにより自然な URL 構造を保てる。

### 10.3 生成 CLI 優先、preprocessor は optional

初期フェーズは次で十分。

1. `facts/*.yaml` を読み込む
2. `src/` 以下に Markdown を生成する
3. `mdbook build` を実行する

preprocessor は later phase として、以下の用途で検討可能。

- カスタムショートコード
- 共通部品差し込み
- 一部の自動注釈

---

## 11. `book.toml` 方針

例:

```toml
[book]
title = "へえー図鑑"
authors = ["Your Team"]
language = "ja"
multilingual = false
src = "src"

[build]
build-dir = "book"
create-missing = false
use-default-preprocessors = true
extra-watch-dirs = ["facts", "templates", "generated"]

[output.html]
default-theme = "ayu"
git-repository-url = "https://github.com/your-org/hee-book"
edit-url-template = "https://github.com/your-org/hee-book/edit/main/{path}"
site-url = "/zatsu-book/"

[output.markdown]

[preprocessor.links]
[preprocessor.index]
```

補足:

- `extra-watch-dirs` に `facts/` を入れて、データ変更で `serve` が再ビルドされるようにする
- `output.markdown` を有効化して、preprocessor/生成後の Markdown を確認しやすくする
- `use-default-preprocessors = true` のまま始める
- `site-url` は GitHub Pages の **project site** 前提で repo 名込みにする
- `src/404.md` も用意しておくと Pages 公開時の導線が崩れにくい

---

## 12. 運用 CLI 設計 (`factctl`)

### 12.1 主要コマンド

#### `factctl new`

新規テンプレートを生成する。

例:

```bash
factctl new --genre money --title "1円玉の木は特定の木ではない"
```

機能:

- ID 候補生成
- YAML テンプレート作成
- serial 採番
- 最低限の front matter 生成

#### `factctl validate`

入力データの整合性を検証する。

チェック内容:

- 必須項目
- enum 妥当性
- 日付形式
- URL 形式
- 参照先存在
- `id` 重複
- `primary_genre in genres`
- taxonomy 定義外のタグ・ジャンル使用

#### `factctl dedupe`

準重複候補を検出し、レポートを出す。

出力:

- ターミナル表示
- `generated/reports/duplicate_candidates.md`

#### `factctl build-pages`

`src/` 以下を生成する。

出力対象:

- `src/SUMMARY.md`
- `src/README.md`
- `src/all/README.md`
- `src/genres/**/README.md`
- `src/tags/**/README.md`
- `src/facts/**/*.md`
- `src/updates/README.md`

#### `factctl update <id>`

既存レコード更新支援。

機能:

- `updated_at` 更新
- `revision += 1`
- エディタ起動

#### `factctl stale`

ソース確認や古いデータ点検対象を抽出する。

#### `factctl doctor`

総合チェック。

実質的には以下のまとめ。

- validate
- dedupe
- stale
- broken reference
- build dry-run

### 12.2 推奨サブコマンド順序

普段の運用は次で回す。

```bash
factctl new ...
factctl validate
factctl dedupe
factctl build-pages
mdbook build
```

---

## 13. 一覧生成ロジック

### 13.1 生成単位

- 個別ネタページ
- ジャンル別一覧
- タグ別一覧
- 更新一覧
- 全件一覧

### 13.2 並び順

用途ごとに固定する。

- 個別ページ: 単一
- ジャンル一覧: `updated_at desc`, 同点は `id asc`
- 全件一覧: `title asc` と `updated_at desc` の2系統を検討
- 更新一覧: `updated_at desc`

### 13.3 非公開の扱い

`status != published` のものは通常の公開ページに出さない。  
ただし `generated/reports/unpublished.md` には出す。

---

## 14. 編集ルール

### 14.1 記述ルール

- タイトルは断定文にする
- `summary` は 80〜120 文字目安
- `claim` は事実主張を簡潔に一文で表す
- `explanation` は背景や意外性の説明を書く
- 一次情報を優先する

### 14.2 1件に含めないこと

- 複数の主張を1レコードに詰め込むこと
- 根拠のない雑学
- 出典の薄い伝聞
- テーマが広すぎるまとめ記事

### 14.3 更新と新規の判断

#### 更新でよい

- 要約改善
- 誤字修正
- 根拠 URL 差し替え
- 補足追加

#### 新規にすべき

- 主張の中心が変わる
- タイトルが別主張になる
- 意外性の核が別物になる

---

## 15. スキーマ例（JSON Schema の考え方）

`schemas/fact.schema.json` で最低限以下を縛る。

- `id`: `^[a-z0-9]+(?:-[a-z0-9]+)*$`
- `primary_genre`: 文字列、taxonomy 定義済み
- `status`: enum
- `genres`: 1件以上
- `sources`: 1件以上
- `revision`: integer, minimum 1
- 日付項目: `YYYY-MM-DD`

簡略例:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": [
    "id",
    "title",
    "primary_genre",
    "genres",
    "summary",
    "claim",
    "sources",
    "status",
    "created_at",
    "updated_at",
    "revision"
  ],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^[a-z0-9]+(?:-[a-z0-9]+)*$"
    },
    "status": {
      "type": "string",
      "enum": ["draft", "published", "duplicate", "superseded", "archived"]
    },
    "genres": {
      "type": "array",
      "minItems": 1,
      "items": { "type": "string" }
    },
    "sources": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["url", "publisher", "kind", "accessed_at"]
      }
    },
    "revision": {
      "type": "integer",
      "minimum": 1
    }
  }
}
```

---

## 16. Rust 実装方針

### 16.1 推奨クレート

- `serde`, `serde_yaml`, `serde_json`
- `anyhow`, `thiserror`
- `clap`
- `walkdir`
- `regex`
- `unicode-normalization`
- `handlebars` または `minijinja`
- `chrono`
- `url`
- `similar` または独自 trigram 実装

### 16.2 モジュール分割案

```text
tools/factctl/src/
├─ main.rs
├─ cli.rs
├─ model.rs
├─ schema.rs
├─ load.rs
├─ normalize.rs
├─ dedupe.rs
├─ validate.rs
├─ render.rs
├─ summary.rs
├─ reports.rs
└─ commands/
   ├─ new.rs
   ├─ validate.rs
   ├─ dedupe.rs
   ├─ build_pages.rs
   ├─ update.rs
   ├─ stale.rs
   └─ doctor.rs
```

### 16.3 コア型の例

```rust
pub struct Fact {
    pub id: String,
    pub title: String,
    pub primary_genre: String,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub summary: String,
    pub claim: String,
    pub explanation: Option<String>,
    pub sources: Vec<Source>,
    pub status: FactStatus,
    pub created_at: String,
    pub updated_at: String,
    pub revision: u32,
    pub aliases: Vec<String>,
    pub duplicate_of: Option<String>,
    pub supersedes: Option<String>,
    pub canonical: bool,
    pub importance: Option<f32>,
}
```

---

## 17. 生成戦略

### 17.1 テンプレートベース生成

`Handlebars` か `MiniJinja` を使い、以下をテンプレート化する。

- 個別ページ
- ジャンル一覧ページ
- タグ一覧ページ
- トップページ
- 更新一覧
- レポート

### 17.2 生成物の責務

#### `src/` 以下

mdBook の入力として使う。  
公開に必要なファイルのみ置く。

#### `generated/` 以下

人間向け補助レポートや一時生成物を置く。  
公開対象ではない。

### 17.3 Git 管理方針

- `facts/` は正本としてコミットする
- `src/` と `generated/` は生成物だが、CI の再現性確認のためコミットする
- `book/` は deploy 用ビルド成果物であり、Git にはコミットしない
- GitHub Pages への公開は `gh-pages` ブランチではなく GitHub Actions artifact デプロイを使う

---

## 18. CI 設計

### 18.1 必須ジョブ

1. Rust format / lint
2. `factctl validate`
3. `factctl dedupe --fail-on-high-confidence-duplicate`
4. `factctl build-pages`
5. `git diff --exit-code src generated`
6. `mdbook build`
7. リンクチェック（optional）

### 18.2 GitHub Actions 例

```yaml
name: ci

on:
  pull_request:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: swatinem/rust-cache@v2

      - name: Install mdBook
        run: cargo install mdbook --locked

      - name: Validate facts
        run: cargo run -p factctl -- validate

      - name: Detect duplicates
        run: cargo run -p factctl -- dedupe --fail-on-high-confidence-duplicate

      - name: Build pages
        run: cargo run -p factctl -- build-pages

      - name: Ensure generated files are committed
        run: git diff --exit-code -- src generated

      - name: Build mdBook
        run: mdbook build
```

### 18.3 PR ルール

- `facts/` 変更時は `generated/` と `src/` の差分も含める
- CI 緑必須
- 高信頼 duplicate 候補がある場合はマージ不可
- `book/` の差分は PR に含めない

---

## 19. 公開・配信

### 19.1 公開方法

- GitHub Pages
- Cloudflare Pages
- S3 + CDN

初期は **GitHub Pages の project site + GitHub Actions デプロイ** で十分。

### 19.2 デプロイフロー

- main マージ
- GitHub Actions で `factctl build-pages`
- GitHub Actions で `mdbook build`
- `book/` を artifact として upload
- `deploy-pages` で Pages へ公開

補足:

- 公開 URL は `https://<owner>.github.io/zatsu-book/`
- `book.toml` の `site-url` も `/zatsu-book/` に合わせる
- Pages workflow では `pages: write` と `id-token: write` を付与する
- 初期段階では独自ドメインを使わない
- `gh-pages` ブランチへ成果物をコミットする方式は採用しない

---

## 20. 将来拡張

### 20.1 検索性強化

- タグ synonyms
- alias 検索
- related facts 自動推薦

### 20.2 editorial workflow

- reviewer フィールド追加
- 確認済みフラグ
- 信頼度スコア

### 20.3 ソース点検自動化

- URL 生存確認
- 更新日ヘッダ監視
- official ドメイン許可リスト

### 20.4 UI 拡張

- 「ランダムへえー」ボタン
- 難易度別導線
- 子ども向け / 大人向け切替

---

## 21. 実装フェーズ提案

### Phase 1: MVP

- YAML ローダー
- スキーマ検証
- 個別ページ生成
- ジャンル一覧生成
- `SUMMARY.md` 自動生成
- `mdbook build` 成功

### Phase 2: 運用強化

- dedupe 実装
- 更新一覧
- タグ一覧
- stale レポート
- `factctl update`

### Phase 3: 品質強化

- 高度な準重複判定
- ソースチェック自動化
- 関連ネタ推薦
- テンプレート洗練

---

## 22. Codex への実装依頼単位

Codex には次の順で依頼すると実装しやすい。

1. ワークスペース初期化
   - Cargo workspace
   - `tools/factctl` 作成
   - `book.toml` 作成
   - `config/taxonomy.yaml` 作成
   - `src/` の最小構成作成

2. データモデルとローダー
   - `Fact`, `Source`, `FactStatus`, `Taxonomy`
   - YAML 読み込み
   - taxonomy 読み込み
   - ディレクトリ走査

3. validate 実装
   - 必須項目
   - enum
   - `primary_genre` / taxonomy 整合性
   - URL / date / 参照整合性

4. build-pages 実装
   - 個別ページ
   - ジャンル一覧
   - トップページ
   - `SUMMARY.md`

5. dedupe 実装
   - claim 正規化
   - trigram Jaccard
   - レポート出力

6. CI 実装
   - GitHub Actions
   - mdBook build
   - GitHub Pages deploy

7. update/stale/doctor 実装

---

## 23. 受け入れ条件

以下を満たしたら MVP 完了とする。

- `facts/*.yaml` から mdBook 用 Markdown を自動生成できる
- `src/SUMMARY.md` を手編集しなくてよい
- `factctl validate` で基本エラーを検出できる
- `factctl dedupe` で重複候補を提示できる
- `mdbook build` が成功し、ジャンル別に閲覧できる
- `draft` が公開ページに出ない
- 1件の更新で `revision` と `updated_at` を上げられる
- GitHub Pages の project site で崩れず公開できる

---

## 24. 非採用案

### 24.1 Markdown を直接正本にする案

却下理由:

- 重複判定しづらい
- 構造が崩れやすい
- 一覧生成が散らかる
- 更新履歴と公開ページが密結合になる

### 24.2 最初から mdBook preprocessor 中心にする案

保留理由:

- デバッグが重くなりやすい
- 初期実装では CLI 生成の方が見通しがよい
- 将来的に必要なら移行できる

---

## 25. 実装時の注意点

- `src/` は生成物として扱うなら、手編集ファイルと分離する
- 生成順で差分が不安定にならないよう、**常にソート**する
- YAML の書式揺れで不要 diff が増えないよう、保存時フォーマットを統一する
- URL 正規化は query string の扱いに注意する
- 日本語テキスト正規化は過剰にやりすぎない
- `duplicate_of` / `supersedes` が循環しないよう検証する
- taxonomy の slug を表示名に流用しない
- Pages 公開前提のため `site-url` と 404 導線を早い段階で確認する

---

## 26. 参考実装の最小 Makefile 例

```makefile
.PHONY: validate build-pages book doctor serve

validate:
	cargo run -p factctl -- validate

build-pages:
	cargo run -p factctl -- build-pages

book: build-pages
	mdbook build

doctor:
	cargo run -p factctl -- doctor

serve: build-pages
	mdbook serve --open
```

---

## 27. まとめ

本設計の核は、**「ネタデータを正本にし、mdBook は表示層として使う」**ことにある。  
これにより、追加・更新・重複防止・一覧生成・公開をきれいに分離できる。

最終的な方針は以下。

- 正本は `facts/*.yaml`
- 1ネタ = 1主張
- `genres` / `tags` は内部 slug、表示名は taxonomy で管理
- `primary_genre` で保存先と ID の先頭 genre を固定する
- `factctl` が validate / dedupe / build-pages を担う
- `src/` は生成物
- `src/` と `generated/` はコミットし、`book/` はコミットしない
- `mdbook build` で静的公開
- GitHub Actions で CI と Pages 公開を担保

この構成なら、初期は小さく始めつつ、件数が増えても破綻しにくい。

---

## 28. 参考情報

設計の前提として参照した mdBook の公式ドキュメント:

- Creating a book
- SUMMARY.md
- Configuring Preprocessors
- Preprocessors (for developers)
- General configuration
- Renderers
