use crate::load::{LoadedFact, discover_fact_paths, load_fact_from};
use crate::model::{Fact, FactStatus};
use crate::normalize::{normalize_claim, normalize_text, trigram_jaccard};
use anyhow::{Result, bail};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const FACTS_DIR: &str = "facts";
const REPORT_PATH: &str = "generated/reports/duplicate_candidates.md";
const NEAR_DUPLICATE_THRESHOLD: f64 = 0.40;
const HIGH_CONFIDENCE_THRESHOLD: f64 = 0.94;

pub fn run(root: &Path, fail_on_high_confidence_duplicate: bool) -> Result<()> {
    let facts = load_facts_allow_duplicate_ids(root)?;
    let analysis = analyze(&facts);
    write_report(root, &analysis)?;
    print_summary(&analysis);

    if fail_on_high_confidence_duplicate && analysis.high_confidence_count() > 0 {
        bail!(
            "found {} high-confidence duplicate candidate(s); see {}",
            analysis.high_confidence_count(),
            REPORT_PATH
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ExactDuplicateKind {
    Id,
    Claim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExactDuplicateGroup {
    kind: ExactDuplicateKind,
    key: String,
    members: Vec<ReportMember>,
}

#[derive(Debug, Clone, PartialEq)]
struct NearDuplicateCandidate {
    left: ReportMember,
    right: ReportMember,
    title_score: f64,
    summary_score: f64,
    claim_score: f64,
    overall_score: f64,
    high_confidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportMember {
    id: String,
    path: PathBuf,
    status: FactStatus,
}

#[derive(Debug, Clone, PartialEq)]
struct DedupeAnalysis {
    fact_count: usize,
    exact_duplicates: Vec<ExactDuplicateGroup>,
    near_duplicates: Vec<NearDuplicateCandidate>,
}

#[derive(Debug)]
struct NormalizedFact<'a> {
    loaded: &'a LoadedFact,
    title_variants: Vec<String>,
    summary: String,
    claim: String,
}

impl<'a> NormalizedFact<'a> {
    fn new(loaded: &'a LoadedFact) -> Self {
        Self {
            title_variants: title_variants(&loaded.fact),
            summary: normalize_text(&loaded.fact.summary),
            claim: normalize_claim(&loaded.fact.claim),
            loaded,
        }
    }

    fn member(&self) -> ReportMember {
        ReportMember {
            id: self.loaded.fact.id.clone(),
            path: self.loaded.path.clone(),
            status: self.loaded.fact.status.clone(),
        }
    }

    fn path_key(&self) -> String {
        self.loaded.path.display().to_string()
    }
}

impl DedupeAnalysis {
    fn high_confidence_count(&self) -> usize {
        self.near_duplicates
            .iter()
            .filter(|candidate| candidate.high_confidence)
            .count()
            + self.exact_duplicates.len()
    }
}

fn load_facts_allow_duplicate_ids(root: &Path) -> Result<Vec<LoadedFact>> {
    let facts_root = root.join(FACTS_DIR);
    let mut facts = Vec::new();
    for path in discover_fact_paths(&facts_root)? {
        facts.push(load_fact_from(&path)?);
    }
    facts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.fact.id.cmp(&right.fact.id))
    });
    Ok(facts)
}

fn analyze(facts: &[LoadedFact]) -> DedupeAnalysis {
    let normalized = facts.iter().map(NormalizedFact::new).collect::<Vec<_>>();
    let exact_duplicates = collect_exact_duplicates(&normalized);
    let exact_pairs = exact_pair_keys(&exact_duplicates);
    let near_duplicates = collect_near_duplicates(&normalized, &exact_pairs);

    DedupeAnalysis {
        fact_count: facts.len(),
        exact_duplicates,
        near_duplicates,
    }
}

fn collect_exact_duplicates(facts: &[NormalizedFact<'_>]) -> Vec<ExactDuplicateGroup> {
    let mut groups = Vec::new();
    groups.extend(collect_groups(facts, ExactDuplicateKind::Id, |fact| {
        Some(fact.loaded.fact.id.clone())
    }));
    groups.extend(collect_groups(facts, ExactDuplicateKind::Claim, |fact| {
        (!fact.claim.is_empty()).then(|| fact.claim.clone())
    }));
    groups.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.members[0].path.cmp(&right.members[0].path))
    });
    groups
}

fn collect_groups(
    facts: &[NormalizedFact<'_>],
    kind: ExactDuplicateKind,
    key_fn: impl Fn(&NormalizedFact<'_>) -> Option<String>,
) -> Vec<ExactDuplicateGroup> {
    let mut grouped = BTreeMap::<String, Vec<ReportMember>>::new();
    for fact in facts {
        if let Some(key) = key_fn(fact) {
            grouped.entry(key).or_default().push(fact.member());
        }
    }

    grouped
        .into_iter()
        .filter_map(|(key, mut members)| {
            if members.len() < 2 {
                return None;
            }
            members.sort_by(|left, right| {
                left.path
                    .cmp(&right.path)
                    .then_with(|| left.id.cmp(&right.id))
            });
            Some(ExactDuplicateGroup {
                kind: kind.clone(),
                key,
                members,
            })
        })
        .collect()
}

fn exact_pair_keys(groups: &[ExactDuplicateGroup]) -> BTreeSet<(String, String)> {
    let mut pairs = BTreeSet::new();
    for group in groups {
        for (index, left) in group.members.iter().enumerate() {
            for right in group.members.iter().skip(index + 1) {
                let left = left.path.display().to_string();
                let right = right.path.display().to_string();
                if left < right {
                    pairs.insert((left, right));
                } else {
                    pairs.insert((right, left));
                }
            }
        }
    }
    pairs
}

fn collect_near_duplicates(
    facts: &[NormalizedFact<'_>],
    exact_pairs: &BTreeSet<(String, String)>,
) -> Vec<NearDuplicateCandidate> {
    let mut candidates = Vec::new();
    for (index, left) in facts.iter().enumerate() {
        if matches!(left.loaded.fact.status, FactStatus::Duplicate) {
            continue;
        }

        for right in facts.iter().skip(index + 1) {
            if matches!(right.loaded.fact.status, FactStatus::Duplicate) {
                continue;
            }

            let pair_key = ordered_pair(left.path_key(), right.path_key());
            if exact_pairs.contains(&pair_key) {
                continue;
            }

            if let Some(candidate) = near_duplicate_candidate(left, right) {
                candidates.push(candidate);
            }
        }
    }

    candidates.sort_by(|left, right| {
        right
            .high_confidence
            .cmp(&left.high_confidence)
            .then_with(|| {
                right
                    .overall_score
                    .partial_cmp(&left.overall_score)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.left.id.cmp(&right.left.id))
            .then_with(|| left.right.id.cmp(&right.right.id))
    });
    candidates
}

fn near_duplicate_candidate(
    left: &NormalizedFact<'_>,
    right: &NormalizedFact<'_>,
) -> Option<NearDuplicateCandidate> {
    let title_score = variant_similarity(&left.title_variants, &right.title_variants);
    let summary_score = trigram_jaccard(&left.summary, &right.summary);
    let claim_score = trigram_jaccard(&left.claim, &right.claim);
    let overall_score = (claim_score * 0.55) + (title_score * 0.30) + (summary_score * 0.15);
    let high_confidence =
        (overall_score >= HIGH_CONFIDENCE_THRESHOLD && claim_score >= 0.90 && title_score >= 0.90)
            || (claim_score >= 0.96 && title_score >= 0.92);

    if overall_score < NEAR_DUPLICATE_THRESHOLD && claim_score < 0.75 {
        return None;
    }

    Some(NearDuplicateCandidate {
        left: left.member(),
        right: right.member(),
        title_score,
        summary_score,
        claim_score,
        overall_score,
        high_confidence,
    })
}

fn variant_similarity(left: &[String], right: &[String]) -> f64 {
    let mut best = 0.0_f64;
    for left in left {
        for right in right {
            best = best.max(trigram_jaccard(left, right));
        }
    }
    best
}

fn title_variants(fact: &Fact) -> Vec<String> {
    let mut variants = Vec::new();
    let title = normalize_text(&fact.title);
    if !title.is_empty() {
        variants.push(title);
    }
    for alias in &fact.aliases {
        let alias = normalize_text(alias);
        if !alias.is_empty() && !variants.contains(&alias) {
            variants.push(alias);
        }
    }
    variants
}

fn ordered_pair(left: String, right: String) -> (String, String) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

fn write_report(root: &Path, analysis: &DedupeAnalysis) -> Result<()> {
    let path = root.join(REPORT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, render_report(analysis))?;
    Ok(())
}

fn print_summary(analysis: &DedupeAnalysis) {
    println!(
        "dedupe complete: {} facts, {} exact duplicate groups, {} near duplicate pairs",
        analysis.fact_count,
        analysis.exact_duplicates.len(),
        analysis.near_duplicates.len()
    );

    for group in &analysis.exact_duplicates {
        println!(
            "exact duplicate [{}] {} => {}",
            group_kind_label(&group.kind),
            summarize_key(&group.key),
            group
                .members
                .iter()
                .map(|member| member.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let high_confidence_candidates = analysis
        .near_duplicates
        .iter()
        .filter(|candidate| candidate.high_confidence)
        .collect::<Vec<_>>();

    for candidate in &high_confidence_candidates {
        println!(
            "near duplicate [{}] {:.2}: {} <-> {} (claim {:.2}, title {:.2}, summary {:.2})",
            if candidate.high_confidence {
                "high"
            } else {
                "candidate"
            },
            candidate.overall_score,
            candidate.left.id,
            candidate.right.id,
            candidate.claim_score,
            candidate.title_score,
            candidate.summary_score
        );
    }

    let candidate_only_count = analysis.near_duplicates.len() - high_confidence_candidates.len();
    if candidate_only_count > 0 {
        println!(
            "near duplicate candidate pairs below fail threshold: {}",
            candidate_only_count
        );
    }

    println!("report: {}", REPORT_PATH);
}

fn render_report(analysis: &DedupeAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# Duplicate Candidates\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- Facts scanned: {}\n", analysis.fact_count));
    out.push_str(&format!(
        "- Exact duplicate groups: {}\n",
        analysis.exact_duplicates.len()
    ));
    out.push_str(&format!(
        "- Near duplicate pairs: {}\n",
        analysis.near_duplicates.len()
    ));
    out.push_str(&format!(
        "- High-confidence findings: {}\n\n",
        analysis.high_confidence_count()
    ));

    out.push_str("## Exact Duplicates\n\n");
    if analysis.exact_duplicates.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for group in &analysis.exact_duplicates {
            out.push_str(&format!(
                "### {}: `{}`\n\n",
                group_kind_label(&group.kind),
                summarize_key(&group.key)
            ));
            for member in &group.members {
                out.push_str(&format!(
                    "- `{}` ({}) [{}]\n",
                    member.id,
                    member.status.as_str(),
                    member.path.display()
                ));
            }
            out.push('\n');
        }
    }

    out.push_str("## Near Duplicates\n\n");
    if analysis.near_duplicates.is_empty() {
        out.push_str("_None._\n");
    } else {
        for candidate in &analysis.near_duplicates {
            out.push_str(&format!(
                "- {} `{}` <-> `{}` | overall `{:.2}` | claim `{:.2}` | title/alias `{:.2}` | summary `{:.2}`\n",
                if candidate.high_confidence { "**high**" } else { "**candidate**" },
                candidate.left.id,
                candidate.right.id,
                candidate.overall_score,
                candidate.claim_score,
                candidate.title_score,
                candidate.summary_score
            ));
        }
    }

    out
}

fn group_kind_label(kind: &ExactDuplicateKind) -> &'static str {
    match kind {
        ExactDuplicateKind::Id => "ID",
        ExactDuplicateKind::Claim => "Normalized claim",
    }
}

fn summarize_key(key: &str) -> String {
    const MAX_LEN: usize = 96;
    if key.chars().count() <= MAX_LEN {
        key.to_owned()
    } else {
        let truncated = key.chars().take(MAX_LEN - 1).collect::<String>();
        format!("{truncated}…")
    }
}

trait FactStatusLabel {
    fn as_str(&self) -> &'static str;
}

impl FactStatusLabel for FactStatus {
    fn as_str(&self) -> &'static str {
        match self {
            FactStatus::Draft => "draft",
            FactStatus::Published => "published",
            FactStatus::Duplicate => "duplicate",
            FactStatus::Superseded => "superseded",
            FactStatus::Archived => "archived",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Editorial, Source, SourceKind};
    use chrono::NaiveDate;
    use tempfile::TempDir;

    fn sample_loaded_fact(
        id: &str,
        title: &str,
        summary: &str,
        claim: &str,
        aliases: &[&str],
        url: &str,
        path: &str,
    ) -> LoadedFact {
        LoadedFact {
            path: PathBuf::from(path),
            fact: Fact {
                id: id.to_owned(),
                title: title.to_owned(),
                primary_genre: "money".to_owned(),
                genres: vec!["money".to_owned()],
                tags: vec!["currency".to_owned()],
                summary: summary.to_owned(),
                claim: claim.to_owned(),
                explanation: None,
                sources: vec![Source {
                    id: "source-1".to_owned(),
                    url: url.to_owned(),
                    title: "Source".to_owned(),
                    publisher: "Publisher".to_owned(),
                    kind: SourceKind::Official,
                    accessed_at: NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
                    quoted_fact: None,
                }],
                status: FactStatus::Published,
                created_at: NaiveDate::from_ymd_opt(2026, 3, 14).expect("valid date"),
                updated_at: NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
                revision: 1,
                aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
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

    fn temp_repo() -> TempDir {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("facts/money")).expect("facts dir");
        temp
    }

    #[test]
    fn detects_exact_duplicate_claims() {
        let facts = vec![
            sample_loaded_fact(
                "money-001-yen-tree",
                "1円玉の木は特定の木ではない",
                "summary 1",
                "1円玉の木には特定の樹種名がない",
                &[],
                "https://example.com/1",
                "facts/money/money-001-yen-tree.yaml",
            ),
            sample_loaded_fact(
                "money-002-yen-no-tree",
                "1円玉の木は何の木か決まっていない",
                "summary 2",
                "1円玉の木には特定の樹種名がない",
                &[],
                "https://example.com/2",
                "facts/money/money-002-yen-no-tree.yaml",
            ),
        ];

        let analysis = analyze(&facts);
        assert_eq!(analysis.exact_duplicates.len(), 1);
        assert_eq!(analysis.exact_duplicates[0].kind, ExactDuplicateKind::Claim);
        assert!(analysis.near_duplicates.is_empty());
    }

    #[test]
    fn detects_near_duplicate_candidates() {
        let facts = vec![
            sample_loaded_fact(
                "money-001-yen-tree",
                "1円玉の木は特定の樹種名がない",
                "造幣局は1円玉の木を若木として説明している。",
                "1円玉のデザインの木には具体的な木の名前はない",
                &["1円玉の木は何の木？"],
                "https://example.com/1",
                "facts/money/money-001-yen-tree.yaml",
            ),
            sample_loaded_fact(
                "money-002-yen-no-tree",
                "1円玉の木に特定の樹種名はない",
                "造幣局は1円玉の木を若木と説明している。",
                "1円玉のデザインの木に具体的な名前は設定されていない",
                &["1円玉の木に特定の木の名前はない"],
                "https://example.com/2",
                "facts/money/money-002-yen-no-tree.yaml",
            ),
        ];

        let analysis = analyze(&facts);
        assert_eq!(analysis.exact_duplicates.len(), 0);
        assert_eq!(analysis.near_duplicates.len(), 1);
        assert!(analysis.near_duplicates[0].overall_score >= NEAR_DUPLICATE_THRESHOLD);
    }

    #[test]
    fn writes_report_when_no_candidates_are_found() {
        let temp = temp_repo();
        fs::write(
            temp.path()
                .join("facts/money/money-001-yen-tree-not-specific.yaml"),
            include_str!(
                "../tests/fixtures/facts/valid/money/money-001-yen-tree-not-specific.yaml"
            ),
        )
        .expect("seed fact");

        run(temp.path(), false).expect("dedupe should succeed");

        let report = fs::read_to_string(temp.path().join(REPORT_PATH)).expect("read report");
        assert!(report.contains("Exact duplicate groups: 0"));
        assert!(report.contains("Near duplicate pairs: 0"));
    }

    #[test]
    fn shared_primary_source_url_alone_is_not_an_exact_duplicate() {
        let facts = vec![
            sample_loaded_fact(
                "money-0001",
                "1円玉は1グラム",
                "summary 1",
                "1円玉は1グラムである",
                &[],
                "https://example.com/faq",
                "facts/money/money-0001.yaml",
            ),
            sample_loaded_fact(
                "money-0002",
                "5円玉には穴がある",
                "summary 2",
                "5円玉には穴がある",
                &[],
                "https://example.com/faq",
                "facts/money/money-0002.yaml",
            ),
        ];

        let analysis = analyze(&facts);

        assert!(analysis.exact_duplicates.is_empty());
    }

    #[test]
    fn patterned_sibling_facts_are_candidates_but_not_high_confidence() {
        let facts = vec![
            sample_loaded_fact(
                "weather-0064",
                "気象庁は九州南部の梅雨入り・梅雨明けを速報として発表するが、後で確定値が変わることがある",
                "気象庁は九州南部の梅雨入り・梅雨明けを速報として発表するが、後で確定値が変わることがあると公式FAQ・解説で扱われている。",
                "気象庁は九州南部の梅雨入り・梅雨明けを速報として発表するが、後で確定値が変わることがある",
                &[],
                "https://example.com/faq1",
                "facts/weather/weather-0064.yaml",
            ),
            sample_loaded_fact(
                "weather-0065",
                "気象庁は九州北部の梅雨入り・梅雨明けを速報として発表するが、後で確定値が変わることがある",
                "気象庁は九州北部の梅雨入り・梅雨明けを速報として発表するが、後で確定値が変わることがあると公式FAQ・解説で扱われている。",
                "気象庁は九州北部の梅雨入り・梅雨明けを速報として発表するが、後で確定値が変わることがある",
                &[],
                "https://example.com/faq1",
                "facts/weather/weather-0065.yaml",
            ),
        ];

        let analysis = analyze(&facts);

        assert_eq!(analysis.near_duplicates.len(), 1);
        assert!(!analysis.near_duplicates[0].high_confidence);
    }
}
