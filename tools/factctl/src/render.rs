use crate::load::{self, LoadedFact};
use crate::model::{FactStatus, SourceKind, Taxonomy};
use crate::terms::{ResolvedTerm, TaxonomyKind, resolve_terms_unique};
use crate::validate;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct TermPage<'a> {
    slug: String,
    label: String,
    facts: Vec<&'a LoadedFact>,
}

pub fn build_pages(root: &Path) -> Result<()> {
    build_pages_with_mode(root, BuildMode::Write)
}

pub fn build_pages_dry_run(root: &Path) -> Result<()> {
    build_pages_with_mode(root, BuildMode::DryRun)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildMode {
    Write,
    DryRun,
}

fn build_pages_with_mode(root: &Path, mode: BuildMode) -> Result<()> {
    let report = validate::validate_repository(root);
    if !report.is_valid() {
        eprintln!("build-pages requires valid input data:");
        for issue in report.issues {
            eprintln!("- {}", issue.render());
        }
        bail!("build-pages aborted because validation failed");
    }

    let taxonomy = load::load_taxonomy(root)?;
    let facts = load::load_facts(root)?;
    let mut published = facts
        .facts()
        .iter()
        .filter(|loaded| loaded.fact.status == FactStatus::Published)
        .collect::<Vec<_>>();
    sort_by_updated_desc(&mut published);

    let mut unpublished = facts
        .facts()
        .iter()
        .filter(|loaded| loaded.fact.status != FactStatus::Published)
        .collect::<Vec<_>>();
    sort_by_updated_desc(&mut unpublished);

    let genres = build_term_pages(
        &published,
        &taxonomy,
        |fact| &fact.genres,
        TaxonomyKind::Genre,
    );
    let tags = build_term_pages(&published, &taxonomy, |fact| &fact.tags, TaxonomyKind::Tag);

    let mut outputs = vec![
        (
            "src/README.md".to_owned(),
            render_top_page(&published, &genres, &tags),
        ),
        ("src/all/README.md".to_owned(), render_all_page(&published)),
        (
            "src/genres/README.md".to_owned(),
            render_term_index("ジャンル一覧", &genres),
        ),
        (
            "src/tags/README.md".to_owned(),
            render_term_index("タグ一覧", &tags),
        ),
        (
            "src/updates/README.md".to_owned(),
            render_updates_page(&published),
        ),
        (
            "src/SUMMARY.md".to_owned(),
            render_summary(&published, &genres, &tags),
        ),
        ("src/404.md".to_owned(), render_404_page()),
        (
            "generated/reports/unpublished.md".to_owned(),
            render_unpublished_report(&unpublished),
        ),
    ];

    for fact in &published {
        outputs.push((
            format!("src/facts/{}/{}.md", fact.fact.primary_genre, fact.fact.id),
            render_fact_page(fact, &taxonomy),
        ));
    }

    for genre in &genres {
        outputs.push((
            format!("src/genres/{}/README.md", genre.slug),
            render_term_page("ジャンル", genre, "../../"),
        ));
    }

    for tag in &tags {
        outputs.push((
            format!("src/tags/{}/README.md", tag.slug),
            render_term_page("タグ", tag, "../../"),
        ));
    }

    emit_outputs(root, &outputs, mode)?;

    println!(
        "{}: {} published facts, {} unpublished facts",
        match mode {
            BuildMode::Write => "build-pages ok",
            BuildMode::DryRun => "build-pages dry-run ok",
        },
        published.len(),
        unpublished.len()
    );
    Ok(())
}

fn emit_outputs(root: &Path, outputs: &[(String, String)], mode: BuildMode) -> Result<()> {
    if mode == BuildMode::DryRun {
        return Ok(());
    }

    clean_managed_outputs(root)?;
    for (relative_path, content) in outputs {
        write_markdown(root, relative_path, content.clone())?;
    }
    Ok(())
}

fn clean_managed_outputs(root: &Path) -> Result<()> {
    for relative_dir in [
        "src/all",
        "src/facts",
        "src/genres",
        "src/tags",
        "src/updates",
    ] {
        remove_dir_if_exists(&root.join(relative_dir))?;
    }

    for relative_file in [
        "src/README.md",
        "src/SUMMARY.md",
        "src/404.md",
        "generated/reports/unpublished.md",
    ] {
        remove_file_if_exists(&root.join(relative_file))?;
    }

    Ok(())
}

fn build_term_pages<'a>(
    facts: &[&'a LoadedFact],
    taxonomy: &'a Taxonomy,
    raw_terms: impl Fn(&'a crate::model::Fact) -> &'a [String],
    kind: TaxonomyKind,
) -> Vec<TermPage<'a>> {
    let mut grouped: BTreeMap<String, (String, Vec<&LoadedFact>)> = BTreeMap::new();
    for fact in facts {
        for term in resolve_terms_unique(raw_terms(&fact.fact), taxonomy, kind) {
            grouped
                .entry(term.slug.clone())
                .or_insert_with(|| (term.label.clone(), Vec::new()))
                .1
                .push(*fact);
        }
    }

    let mut pages = grouped
        .into_iter()
        .map(|(slug, (label, mut grouped_facts))| {
            sort_by_updated_desc(&mut grouped_facts);
            TermPage {
                slug,
                label,
                facts: grouped_facts,
            }
        })
        .collect::<Vec<_>>();

    pages.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.slug.cmp(&right.slug))
    });
    pages
}

fn render_top_page(
    facts: &[&LoadedFact],
    genres: &[TermPage<'_>],
    tags: &[TermPage<'_>],
) -> String {
    let mut out = String::new();
    out.push_str("# 雑本\n\n");
    out.push_str("根拠付きの「へえーってなるネタ」を公開するための mdBook です。\n\n");
    out.push_str("## 入口\n\n");
    out.push_str("- [全件一覧](all/index.html)\n");
    out.push_str("- [ジャンル一覧](genres/index.html)\n");
    out.push_str("- [タグ一覧](tags/index.html)\n");
    out.push_str("- [最近更新](updates/index.html)\n\n");

    out.push_str("## 最近更新\n\n");
    if facts.is_empty() {
        out.push_str("現在、公開中のネタはありません。\n\n");
    } else {
        for fact in facts.iter().take(5) {
            out.push_str(&format!(
                "- [{}](facts/{}/{}.md) ({})\n",
                fact.fact.title, fact.fact.primary_genre, fact.fact.id, fact.fact.updated_at
            ));
        }
        out.push('\n');
    }

    out.push_str("## ジャンル\n\n");
    if genres.is_empty() {
        out.push_str("公開中のジャンルはありません。\n\n");
    } else {
        for genre in genres {
            out.push_str(&format!(
                "- [{}](genres/{}/index.html) ({}件)\n",
                genre.label,
                genre.slug,
                genre.facts.len()
            ));
        }
        out.push('\n');
    }

    out.push_str("## タグ\n\n");
    if tags.is_empty() {
        out.push_str("公開中のタグはありません。\n");
    } else {
        for tag in tags.iter().take(10) {
            out.push_str(&format!(
                "- [{}](tags/{}/index.html) ({}件)\n",
                tag.label,
                tag.slug,
                tag.facts.len()
            ));
        }
    }

    out
}

fn render_all_page(facts: &[&LoadedFact]) -> String {
    let mut out = String::new();
    out.push_str("# 全件一覧\n\n");
    out.push_str(&format!("公開中のネタは {} 件です。\n\n", facts.len()));
    if facts.is_empty() {
        out.push_str("現在、公開中のネタはありません。\n");
        return out;
    }

    for fact in facts {
        out.push_str(&format!(
            "- [{}](../facts/{}/{}.md) ({})\n",
            fact.fact.title, fact.fact.primary_genre, fact.fact.id, fact.fact.updated_at
        ));
    }
    out
}

fn render_term_index(title: &str, pages: &[TermPage<'_>]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {title}\n\n"));
    if pages.is_empty() {
        out.push_str("公開中の項目はありません。\n");
        return out;
    }

    for page in pages {
        out.push_str(&format!(
            "- [{}]({}/index.html) ({}件)\n",
            page.label,
            page.slug,
            page.facts.len()
        ));
    }
    out
}

fn render_term_page(kind: &str, page: &TermPage<'_>, fact_link_prefix: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", page.label));
    out.push_str(&format!(
        "{kind} `{}` に属する公開中のネタは {} 件です。\n\n",
        page.slug,
        page.facts.len()
    ));
    if page.facts.is_empty() {
        out.push_str("現在、公開中のネタはありません。\n");
        return out;
    }

    for fact in &page.facts {
        out.push_str(&format!(
            "- [{}]({}facts/{}/{}.md) ({})\n",
            fact.fact.title,
            fact_link_prefix,
            fact.fact.primary_genre,
            fact.fact.id,
            fact.fact.updated_at
        ));
    }
    out
}

fn render_updates_page(facts: &[&LoadedFact]) -> String {
    let mut out = String::new();
    out.push_str("# 最近更新\n\n");
    if facts.is_empty() {
        out.push_str("現在、公開中のネタはありません。\n");
        return out;
    }

    for fact in facts {
        out.push_str(&format!(
            "- [{}](../facts/{}/{}.md) ({})\n",
            fact.fact.title, fact.fact.primary_genre, fact.fact.id, fact.fact.updated_at
        ));
    }
    out
}

fn render_summary(facts: &[&LoadedFact], genres: &[TermPage<'_>], tags: &[TermPage<'_>]) -> String {
    let mut out = String::new();
    out.push_str("# Summary\n\n");
    out.push_str("- [雑本](README.md)\n");
    out.push_str("- [全件一覧](all/README.md)\n");
    for fact in facts {
        out.push_str(&format!(
            "  - [{}](facts/{}/{}.md)\n",
            fact.fact.title, fact.fact.primary_genre, fact.fact.id
        ));
    }
    out.push_str("- [ジャンル一覧](genres/README.md)\n");
    for genre in genres {
        out.push_str(&format!(
            "  - [{}](genres/{}/README.md)\n",
            genre.label, genre.slug
        ));
    }
    out.push_str("- [タグ一覧](tags/README.md)\n");
    for tag in tags {
        out.push_str(&format!(
            "  - [{}](tags/{}/README.md)\n",
            tag.label, tag.slug
        ));
    }
    out.push_str("- [最近更新](updates/README.md)\n");
    out
}

fn render_fact_page(fact: &LoadedFact, taxonomy: &Taxonomy) -> String {
    let mut out = String::new();
    let resolved_genres = resolve_terms_unique(&fact.fact.genres, taxonomy, TaxonomyKind::Genre);
    let resolved_tags = resolve_terms_unique(&fact.fact.tags, taxonomy, TaxonomyKind::Tag);
    out.push_str(&format!("# {}\n\n", fact.fact.title));
    out.push_str("## 要点\n\n");
    out.push_str(&fact.fact.summary);
    out.push_str("\n\n## 主張\n\n");
    out.push_str(&fact.fact.claim);
    out.push_str("\n\n## 解説\n\n");
    out.push_str(fact.fact.explanation.as_deref().unwrap_or("（未記入）"));
    out.push_str("\n\n## 根拠\n\n");
    for source in &fact.fact.sources {
        let mut line = format!(
            "- [{}]({}) / {} / {} / 最終確認日: {}",
            source.title,
            source.url,
            source.publisher,
            source_kind_label(&source.kind),
            source.accessed_at
        );
        if let Some(quoted_fact) = &source.quoted_fact {
            line.push_str(&format!(" / 引用要点: {}", quoted_fact));
        }
        out.push_str(&line);
        out.push('\n');
    }

    out.push_str("\n## ジャンル\n\n");
    for ResolvedTerm { slug, label } in &resolved_genres {
        out.push_str(&format!("- [{}](../../genres/{}/index.html)\n", label, slug));
    }

    out.push_str("\n## タグ\n\n");
    if resolved_tags.is_empty() {
        out.push_str("タグはありません。\n");
    } else {
        for ResolvedTerm { slug, label } in &resolved_tags {
            out.push_str(&format!("- [{}](../../tags/{}/index.html)\n", label, slug));
        }
    }

    out.push_str("\n## メタデータ\n\n");
    out.push_str(&format!("- ID: `{}`\n", fact.fact.id));
    out.push_str(&format!("- 作成日: {}\n", fact.fact.created_at));
    out.push_str(&format!("- 更新日: {}\n", fact.fact.updated_at));
    out.push_str(&format!("- Revision: {}\n", fact.fact.revision));
    out
}

fn render_unpublished_report(facts: &[&LoadedFact]) -> String {
    let mut out = String::new();
    out.push_str("# 非公開レコード\n\n");
    if facts.is_empty() {
        out.push_str("現在、非公開レコードはありません。\n");
        return out;
    }

    for fact in facts {
        out.push_str(&format!(
            "- `{}` / {} / {} / 更新日: {}\n",
            fact.fact.id,
            fact.fact.title,
            status_label(&fact.fact.status),
            fact.fact.updated_at
        ));
    }
    out
}

fn render_404_page() -> String {
    let mut out = String::new();
    out.push_str("# ページが見つかりません\n\n");
    out.push_str("指定されたページは存在しないか、移動しました。\n\n");
    out.push_str("- [トップページ](index.html)\n");
    out.push_str("- [全件一覧](all/index.html)\n");
    out.push_str("- [ジャンル一覧](genres/index.html)\n");
    out
}

fn write_markdown(root: &Path, relative_path: &str, content: String) -> Result<()> {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }
    fs::write(&path, content).with_context(|| format!("failed to write '{}'", path.display()))
}

fn sort_by_updated_desc(facts: &mut Vec<&LoadedFact>) {
    facts.sort_by(|left, right| {
        right
            .fact
            .updated_at
            .cmp(&left.fact.updated_at)
            .then_with(|| left.fact.id.cmp(&right.fact.id))
    });
}

fn remove_dir_if_exists(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove '{}'", path.display())),
    }
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove '{}'", path.display())),
    }
}

fn source_kind_label(kind: &SourceKind) -> &'static str {
    match kind {
        SourceKind::Official => "official",
        SourceKind::Primary => "primary",
        SourceKind::Secondary => "secondary",
        SourceKind::Media => "media",
        SourceKind::Other => "other",
    }
}

fn status_label(status: &FactStatus) -> &'static str {
    match status {
        FactStatus::Draft => "draft",
        FactStatus::Published => "published",
        FactStatus::Duplicate => "duplicate",
        FactStatus::Superseded => "superseded",
        FactStatus::Archived => "archived",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Editorial, Fact, Source, SourceKind, TaxonomyEntry};
    use chrono::NaiveDate;
    use std::fs;
    use tempfile::TempDir;

    fn sample_taxonomy() -> Taxonomy {
        let genres = [
            (
                "japan".to_owned(),
                TaxonomyEntry {
                    label: "日本".to_owned(),
                },
            ),
            (
                "money".to_owned(),
                TaxonomyEntry {
                    label: "お金".to_owned(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let tags = [
            (
                "coin-design".to_owned(),
                TaxonomyEntry {
                    label: "デザイン".to_owned(),
                },
            ),
            (
                "currency".to_owned(),
                TaxonomyEntry {
                    label: "貨幣".to_owned(),
                },
            ),
        ]
        .into_iter()
        .collect();

        Taxonomy { genres, tags }
    }

    fn sample_fact() -> LoadedFact {
        LoadedFact {
            path: "facts/money/money-001-yen-tree-not-specific.yaml".into(),
            fact: Fact {
                id: "money-001-yen-tree-not-specific".to_owned(),
                title: "1円玉の木は特定の木ではない".to_owned(),
                primary_genre: "money".to_owned(),
                genres: vec!["money".to_owned(), "japan".to_owned()],
                tags: vec!["currency".to_owned(), "coin-design".to_owned()],
                summary: "summary".to_owned(),
                claim: "claim".to_owned(),
                explanation: Some("explanation".to_owned()),
                sources: vec![Source {
                    id: "source-1".to_owned(),
                    url: "https://example.com/source".to_owned(),
                    title: "Source".to_owned(),
                    publisher: "Example".to_owned(),
                    kind: SourceKind::Official,
                    accessed_at: NaiveDate::from_ymd_opt(2026, 3, 14).expect("valid date"),
                    quoted_fact: Some("quoted".to_owned()),
                }],
                status: FactStatus::Published,
                created_at: NaiveDate::from_ymd_opt(2026, 3, 14).expect("valid date"),
                updated_at: NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
                revision: 1,
                aliases: Vec::new(),
                duplicate_of: None,
                supersedes: None,
                canonical: true,
                importance: Some(0.5),
                editorial: Some(Editorial {
                    tone: Some("casual".to_owned()),
                    audience: Some("general".to_owned()),
                    spoiler: false,
                }),
            },
        }
    }

    #[test]
    fn fact_page_uses_taxonomy_labels() {
        let fact = sample_fact();
        let rendered = render_fact_page(&fact, &sample_taxonomy());

        assert!(rendered.contains("[お金](../../genres/money/index.html)"));
        assert!(rendered.contains("[貨幣](../../tags/currency/index.html)"));
        assert!(rendered.contains("引用要点: quoted"));
    }

    #[test]
    fn summary_contains_navigation_links() {
        let fact = sample_fact();
        let taxonomy = sample_taxonomy();
        let published = vec![&fact];
        let genres = build_term_pages(
            &published,
            &taxonomy,
            |loaded| &loaded.genres,
            TaxonomyKind::Genre,
        );
        let tags = build_term_pages(
            &published,
            &taxonomy,
            |loaded| &loaded.tags,
            TaxonomyKind::Tag,
        );
        let rendered = render_summary(&published, &genres, &tags);

        assert!(rendered.contains("- [雑本](README.md)"));
        assert!(rendered.contains(
            "  - [1円玉の木は特定の木ではない](facts/money/money-001-yen-tree-not-specific.md)"
        ));
        assert!(rendered.contains("  - [お金](genres/money/README.md)"));
        assert!(rendered.contains("  - [貨幣](tags/currency/README.md)"));
    }

    #[test]
    fn top_page_and_indexes_use_directory_links_for_index_pages() {
        let fact = sample_fact();
        let taxonomy = sample_taxonomy();
        let published = vec![&fact];
        let genres = build_term_pages(
            &published,
            &taxonomy,
            |loaded| &loaded.genres,
            TaxonomyKind::Genre,
        );
        let tags = build_term_pages(
            &published,
            &taxonomy,
            |loaded| &loaded.tags,
            TaxonomyKind::Tag,
        );

        let top = render_top_page(&published, &genres, &tags);
        let genre_index = render_term_index("ジャンル一覧", &genres);

        assert!(top.contains("[全件一覧](all/index.html)"));
        assert!(top.contains("[ジャンル一覧](genres/index.html)"));
        assert!(top.contains("[タグ一覧](tags/index.html)"));
        assert!(top.contains("[最近更新](updates/index.html)"));
        assert!(top.contains("[お金](genres/money/index.html)"));
        assert!(top.contains("[貨幣](tags/currency/index.html)"));
        assert!(genre_index.contains("[お金](money/index.html)"));
        assert!(!top.contains("README.md"));
        assert!(!genre_index.contains("README.md"));
    }

    fn temp_repo() -> TempDir {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("config")).expect("config dir");
        fs::create_dir_all(temp.path().join("facts/money")).expect("facts dir");
        fs::write(
            temp.path().join("config/taxonomy.yaml"),
            include_str!("../../../config/taxonomy.yaml"),
        )
        .expect("taxonomy");
        fs::write(
            temp.path()
                .join("facts/money/money-001-yen-tree-not-specific.yaml"),
            include_str!(
                "../tests/fixtures/facts/valid/money/money-001-yen-tree-not-specific.yaml"
            ),
        )
        .expect("seed fact");
        temp
    }

    #[test]
    fn dry_run_does_not_write_generated_pages() {
        let temp = temp_repo();

        build_pages_dry_run(temp.path()).expect("dry run should succeed");

        assert!(!temp.path().join("src/README.md").exists());
        assert!(
            !temp
                .path()
                .join("generated/reports/unpublished.md")
                .exists()
        );
    }
}
