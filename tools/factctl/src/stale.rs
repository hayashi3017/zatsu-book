use crate::load::{LoadedFact, load_facts};
use crate::model::{FactStatus, SourceKind};
use anyhow::Result;
use chrono::{Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};

const REPORT_PATH: &str = "generated/reports/stale_sources.md";
const STALE_DAYS: i64 = 180;

pub fn run(root: &Path) -> Result<()> {
    let report = analyze_repository(root, Local::now().date_naive())?;
    write_report(root, &report)?;
    print_summary(&report);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaleSourceFinding {
    fact_id: String,
    fact_title: String,
    fact_status: FactStatus,
    source_id: String,
    source_title: String,
    source_kind: SourceKind,
    accessed_at: NaiveDate,
    age_days: i64,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoOfficialSourceFinding {
    fact_id: String,
    fact_title: String,
    fact_status: FactStatus,
    updated_at: NaiveDate,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaleReport {
    fact_count: usize,
    stale_sources: Vec<StaleSourceFinding>,
    no_official_facts: Vec<NoOfficialSourceFinding>,
}

fn analyze_repository(root: &Path, today: NaiveDate) -> Result<StaleReport> {
    let facts = load_facts(root)?;
    Ok(analyze_facts(facts.facts(), today))
}

fn analyze_facts(facts: &[LoadedFact], today: NaiveDate) -> StaleReport {
    let mut stale_sources = Vec::new();
    let mut no_official_facts = Vec::new();

    for loaded in facts {
        if !loaded
            .fact
            .sources
            .iter()
            .any(|source| matches!(source.kind, SourceKind::Official))
        {
            no_official_facts.push(NoOfficialSourceFinding {
                fact_id: loaded.fact.id.clone(),
                fact_title: loaded.fact.title.clone(),
                fact_status: loaded.fact.status.clone(),
                updated_at: loaded.fact.updated_at,
                path: loaded.path.clone(),
            });
        }

        for source in &loaded.fact.sources {
            let age_days = today.signed_duration_since(source.accessed_at).num_days();
            if age_days > STALE_DAYS {
                stale_sources.push(StaleSourceFinding {
                    fact_id: loaded.fact.id.clone(),
                    fact_title: loaded.fact.title.clone(),
                    fact_status: loaded.fact.status.clone(),
                    source_id: source.id.clone(),
                    source_title: source.title.clone(),
                    source_kind: source.kind.clone(),
                    accessed_at: source.accessed_at,
                    age_days,
                    path: loaded.path.clone(),
                });
            }
        }
    }

    stale_sources.sort_by(|left, right| {
        right
            .age_days
            .cmp(&left.age_days)
            .then_with(|| left.fact_id.cmp(&right.fact_id))
            .then_with(|| left.source_id.cmp(&right.source_id))
    });
    no_official_facts.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.fact_id.cmp(&right.fact_id))
    });

    StaleReport {
        fact_count: facts.len(),
        stale_sources,
        no_official_facts,
    }
}

fn write_report(root: &Path, report: &StaleReport) -> Result<()> {
    let path = root.join(REPORT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, render_report(report))?;
    Ok(())
}

fn print_summary(report: &StaleReport) {
    println!(
        "stale complete: {} facts, {} stale sources, {} facts without official sources",
        report.fact_count,
        report.stale_sources.len(),
        report.no_official_facts.len()
    );
    println!("report: {}", REPORT_PATH);
}

fn render_report(report: &StaleReport) -> String {
    let mut out = String::new();
    out.push_str("# Stale Sources\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- Facts scanned: {}\n", report.fact_count));
    out.push_str(&format!(
        "- Sources older than {} days: {}\n",
        STALE_DAYS,
        report.stale_sources.len()
    ));
    out.push_str(&format!(
        "- Facts without official sources: {}\n\n",
        report.no_official_facts.len()
    ));

    out.push_str("## Sources Older Than Threshold\n\n");
    if report.stale_sources.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for finding in &report.stale_sources {
            out.push_str(&format!(
                "- `{}` / {} / {} / source `{}` {} ({}) / accessed `{}` / {} days old / [{}]\n",
                finding.fact_id,
                finding.fact_title,
                status_label(&finding.fact_status),
                finding.source_id,
                finding.source_title,
                source_kind_label(&finding.source_kind),
                finding.accessed_at,
                finding.age_days,
                finding.path.display()
            ));
        }
        out.push('\n');
    }

    out.push_str("## Facts Without Official Sources\n\n");
    if report.no_official_facts.is_empty() {
        out.push_str("_None._\n");
    } else {
        for finding in &report.no_official_facts {
            out.push_str(&format!(
                "- `{}` / {} / {} / updated `{}` / [{}]\n",
                finding.fact_id,
                finding.fact_title,
                status_label(&finding.fact_status),
                finding.updated_at,
                finding.path.display()
            ));
        }
    }

    out
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
    use crate::model::{Editorial, Fact, Source};
    use chrono::NaiveDate;
    use tempfile::TempDir;

    fn sample_fact(
        id: &str,
        title: &str,
        updated_at: NaiveDate,
        sources: Vec<Source>,
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
                summary: "summary".to_owned(),
                claim: "claim".to_owned(),
                explanation: None,
                sources,
                status: FactStatus::Published,
                created_at: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
                updated_at,
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

    fn sample_source(id: &str, kind: SourceKind, accessed_at: NaiveDate) -> Source {
        Source {
            id: id.to_owned(),
            url: format!("https://example.com/{id}"),
            title: format!("Source {id}"),
            publisher: "Publisher".to_owned(),
            kind,
            accessed_at,
            quoted_fact: None,
        }
    }

    fn temp_repo() -> TempDir {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("facts/money")).expect("facts dir");
        temp
    }

    #[test]
    fn finds_stale_sources_and_missing_official_sources() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date");
        let facts = vec![
            sample_fact(
                "money-001-old-source",
                "old source",
                today,
                vec![sample_source(
                    "official-old",
                    SourceKind::Official,
                    NaiveDate::from_ymd_opt(2025, 9, 1).expect("valid date"),
                )],
                "facts/money/money-001-old-source.yaml",
            ),
            sample_fact(
                "money-002-no-official",
                "no official",
                NaiveDate::from_ymd_opt(2026, 3, 14).expect("valid date"),
                vec![sample_source(
                    "secondary-recent",
                    SourceKind::Secondary,
                    NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date"),
                )],
                "facts/money/money-002-no-official.yaml",
            ),
        ];

        let report = analyze_facts(&facts, today);

        assert_eq!(report.stale_sources.len(), 1);
        assert_eq!(report.stale_sources[0].fact_id, "money-001-old-source");
        assert_eq!(report.no_official_facts.len(), 1);
        assert_eq!(report.no_official_facts[0].fact_id, "money-002-no-official");
    }

    #[test]
    fn writes_report_for_repository() {
        let temp = temp_repo();
        fs::write(
            temp.path()
                .join("facts/money/money-001-yen-tree-not-specific.yaml"),
            include_str!(
                "../tests/fixtures/facts/valid/money/money-001-yen-tree-not-specific.yaml"
            ),
        )
        .expect("seed fact");

        run(temp.path()).expect("stale should succeed");

        let report = fs::read_to_string(temp.path().join(REPORT_PATH)).expect("read report");
        assert!(report.contains("Facts scanned: 1"));
        assert!(report.contains("Facts without official sources: 0"));
    }
}
