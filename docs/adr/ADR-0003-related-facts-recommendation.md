# ADR-0003 関連ネタ推薦の設計方針

- Status: Proposed
- Date: 2026-03-15

## 背景

設計書には「related facts 自動推薦」が将来拡張として挙がっている。Phase 8 では検索性と回遊性を上げたいが、MVP の構成は mdBook + generator であり、重い検索基盤や外部推薦サービスは前提にしていない。

そのため、既存の fact モデルだけで安定に生成できる lightweight な推薦方式が必要になる。

## 決めたいこと

- どの信号を使って関連度を出すか
- 何件まで出すか
- どの status を対象外にするか

## 候補となる信号

- `primary_genre` 一致
- `genres` の重なり
- `tags` の重なり
- `title` / `claim` の正規化後類似度
- `aliases` の重なり
- 主要ソースドメインの一致

## 提案

最初はルールベースの deterministic 推薦にする。

除外条件:

- 自分自身
- `draft`
- `duplicate`
- `superseded`
- `archived`

スコア案:

- `primary_genre` 一致: `+0.25`
- `genres` 共通 1 件ごと: `+0.10`、最大 `+0.20`
- `tags` 共通 1 件ごと: `+0.08`、最大 `+0.24`
- `claim` trigram 類似度: `0.25 * similarity`
- `title` / `aliases` 類似度: `0.06 * similarity`

表示ルール:

- 上位 3 件を表示
- 同点は `updated_at desc`, さらに `id asc`
- 関連度が一定未満なら表示しない

## 実装方針

- `factctl build-pages` の中で関連候補を事前計算する
- 個別ページ末尾に `## 関連するネタ` を生成する
- 計算結果は必要なら `generated/cache/related_facts.json` に保存できるようにする

## 推奨理由

- 既存データモデルの範囲で始められる
- 生成結果が安定する
- CI ノイズが少ない
- 将来 similarity 指標を差し替えても UI 側の変更を最小にできる

## 非推奨とするもの

- embedding ベース推薦を初手で入れる
- 外部 API に依存する推薦
- ユーザー行動ログ前提の推薦

## 保留事項

- `importance` を関連度に混ぜるか
- 共通 source domain をどの程度強く見るか
- tag synonym を関連度計算に含めるか
