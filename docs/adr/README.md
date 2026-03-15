# ADR 一覧

Phase 8 の品質強化に向けた検討メモです。まだ採択前なので、すべて `Proposed` 扱いです。

- [ADR-0001 ソース生存確認の自動化方針](./ADR-0001-source-health-check.md)
- [ADR-0002 official ドメイン許可リストの扱い](./ADR-0002-official-domain-allowlist.md)
- [ADR-0003 関連ネタ推薦の設計方針](./ADR-0003-related-facts-recommendation.md)
- [ADR-0004 alias / tag synonym を検索導線へ反映する方針](./ADR-0004-search-alias-and-tag-synonym.md)
- [ADR-0005 テンプレート文面とトップページ導線の改善方針](./ADR-0005-template-and-top-page-refresh.md)

進め方の推奨順は次です。

1. ADR-0001 と ADR-0002 を先に固める
2. ADR-0004 を決めて検索導線の実装境界を確定する
3. ADR-0003 と ADR-0005 を UI/生成物改善として進める
