# ADR-0001 ソース生存確認の自動化方針

- Status: Proposed
- Date: 2026-03-15

## 背景

Phase 8 では「ソース生存確認の自動化方針」を決める必要がある。現状の `factctl stale` は `accessed_at` の古さと `official` 不足の検出までで、実際の URL 生存確認はしていない。

URL 生存確認をすぐ CI の必須チェックに入れると、外部サイトの瞬断や rate limit によって PR が不安定になる。一方で、完全に手作業のままにすると stale なリンクが長く残りやすい。

## 決めたいこと

- どのタイミングで URL 生存確認を走らせるか
- 失敗結果を warning にするか fail にするか
- レポートをどこへ出すか
- 将来の `doctor` とどう整合させるか

## 選択肢

## 案 A: PR/CI で毎回すべての URL を検査する

利点:

- 常に最新の生存状態を見られる
- 壊れたリンクを merge 前に止めやすい

欠点:

- flaky になりやすい
- 外部サイト都合で CI が赤くなる
- MVP 後の運用コストが高い

## 案 B: 定期実行だけにして、PR ではネットワーク検査しない

利点:

- PR CI が安定する
- 実装が単純

欠点:

- 壊れたリンクの検知が遅れる
- 更新直後の source 差し替えミスを見逃しやすい

## 案 C: 二段構えにする

1. PR/通常 CI ではネットワークアクセスを伴う検査をしない
2. 定期実行または手動実行で source health report を作る
3. 高信頼な障害だけを warning として可視化する

## 提案

案 C を採る。

初期実装方針:

- `factctl stale` は現行どおりローカルデータだけで判定する
- 将来 `factctl source-check` か `factctl stale --check-urls` を追加する
- GitHub Actions では `schedule` と `workflow_dispatch` のみで URL チェックを走らせる
- 初期段階では PR fail 条件にしない
- 結果は `generated/reports/source_health.md` か `generated/reports/stale_sources.md` に統合する

判定レベル:

- `ok`: 200 系
- `redirect`: 300 系。最終到達先を記録
- `soft_fail`: timeout, TLS error, rate limit, 一時的 5xx
- `hard_fail`: 404, 410, DNS 解決不可が一定回数続いた場合

## 推奨の実装順

1. HEAD 優先、失敗時のみ GET fallback
2. 追跡回数を 3 回程度に制限
3. user-agent を明示
4. 結果を cache に保存して短時間の再試行を避ける

cache 候補:

- `generated/cache/source-health.json`

## 影響

- CI の安定性を保ったまま、定期的なリンク点検へ進める
- `doctor` は現状の軽量チェックを維持できる
- 将来 `source-check` を足しても既存運用を崩しにくい

## 保留事項

- 何日連続で `hard_fail` なら issue 化するか
- `Last-Modified` や `ETag` を stale 判定に使うか
- robots.txt やアクセス規約の扱いをどこまで尊重するか
