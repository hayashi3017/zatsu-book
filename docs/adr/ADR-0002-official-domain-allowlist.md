# ADR-0002 official ドメイン許可リストの扱い

- Status: Proposed
- Date: 2026-03-15

## 背景

設計書では official ソースを優先する方針が明記されている。一方で、`source.kind = official` の妥当性は現状では人手判断に依存している。

このままだと、official の名目で実質的に二次情報を混ぜても CLI が気づけない。反対に厳しすぎる reject を入れると、大学の特設サブドメインや地方自治体の委託先 CDN のような実務上の揺れを弾きやすい。

## 決めたいこと

- allowlist をどこで管理するか
- exact match と wildcard をどう扱うか
- validation を error にするか warning にするか

## 選択肢

## 案 A: ドメイン許可リストを持たず、人手レビューだけで運用する

利点:

- 実装が最小
- 例外に強い

欠点:

- `official` の品質が揺れる
- 将来の自動 source health と連携しにくい

## 案 B: strict allowlist にして、未登録ドメインは validate error にする

利点:

- `official` の意味が明確になる
- データ品質は高くなる

欠点:

- 初期整備コストが大きい
- 例外対応が多い時期には運用負荷が高い

## 案 C: allowlist を持ちつつ、初期は warning ベースで運用する

## 提案

案 C を採る。

管理ファイル:

- `config/official_domains.yaml`

初期スキーマ案:

```yaml
rules:
  - pattern: mint.go.jp
    match: exact
    label: 造幣局
    category: government
  - pattern: go.jp
    match: suffix
    label: 日本の官公庁
    category: government
  - pattern: ac.jp
    match: suffix
    label: 日本の大学
    category: education
```

## 運用ルール

- `source.kind = official` かつ allowlist 非一致なら、初期は warning
- `source.kind != official` でも allowlist 一致していれば CLI は提案を出してよい
- suffix match は公開サフィックスに依存せず、単純なホスト末尾一致で始める
- query string は落とさない
- path 単位の細かい制約は初期段階では持たない

## validate への反映案

- `validate` は `warning` と `error` を分けられる形に拡張する余地を残す
- 初期実装では terminal に advisory を出し、CI fail 条件にはしない
- 将来 allowlist が十分育ったら `--strict-official` を追加できるようにする

## 影響

- `official` フラグの意味が徐々に強化される
- `stale` や source health とホスト単位の集計をつなぎやすい
- taxonomy と同じく config-driven な拡張になる

## 保留事項

- 海外官公庁や国際機関のドメインをどこまで含めるか
- `github.io` や `cloudfront.net` 上の official コンテンツをどう扱うか
- warning と error の閾値をいつ切り替えるか
