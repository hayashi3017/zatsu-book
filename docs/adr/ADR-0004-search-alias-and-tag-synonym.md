# ADR-0004 alias / tag synonym を検索導線へ反映する方針

- Status: Proposed
- Date: 2026-03-15

## 背景

現在のデータモデルには `aliases` があり、設計書には `tag synonyms` と `alias 検索` の拡張案がある。一方で、公開サイトは mdBook の標準検索に乗っているため、独自検索 UI を追加しなくても検索ヒット率を上げられる余地がある。

## 決めたいこと

- tag synonym をどこに持つか
- `aliases` と synonyms をどのように mdBook 検索へ載せるか
- visible な UI と search-only なデータをどう分けるか

## 提案

tag synonym は taxonomy に追加し、fact alias と同じ generator パスで扱う。

taxonomy 拡張案:

```yaml
tags:
  currency:
    label: 貨幣
    synonyms:
      - お金
      - コイン
      - 硬貨
```

モデル拡張案:

- `TaxonomyEntry` に `synonyms: Vec<String>` を追加

## 検索導線への反映方法

初期は mdBook 標準検索を活かす。

生成方針:

- 個別 fact ページに `aliases` を `別名` セクションとして明示表示する
- タグページに `synonyms` を `検索キーワード` として表示する
- 必要であれば `search-only` CSS class で視覚上は控えめにする

この方針を推す理由:

- custom preprocessor を入れずに済む
- search index へ自然に載る
- 生成差分が見やすい

## 避けたい案

- JavaScript だけで別検索 index を後付けする
- synonym 情報を HTML comment のみに埋める

理由:

- comment は検索対象に乗らない可能性が高い
- mdBook 標準検索との整合が悪くなる

## 実装ステップ案

1. `TaxonomyEntry` に `synonyms` を追加
2. `validate` で synonym の重複や空文字をチェックする
3. `render` で fact/tag ページへ検索キーワードを出す
4. 必要なら CSS で見た目を調整する

## 保留事項

- genre synonym も同時に扱うか
- `aliases` をトップページの導線にも出すか
- 将来の related facts で synonym をどこまで正規化対象に入れるか
