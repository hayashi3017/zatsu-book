use crate::model::{Editorial, Fact, FactStatus, Source, SourceKind, Taxonomy};
use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const FACTS_DIR: &str = "facts";
const TAXONOMY_PATH: &str = "config/taxonomy.yaml";

#[derive(Debug, Clone)]
pub struct LoadedFact {
    pub path: PathBuf,
    pub fact: Fact,
}

#[derive(Debug, Clone)]
pub struct FactCollection {
    facts: Vec<LoadedFact>,
    by_id: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RawSource {
    #[serde(default)]
    id: Option<String>,
    url: String,
    #[serde(default)]
    title: Option<String>,
    publisher: String,
    kind: SourceKind,
    accessed_at: NaiveDate,
    #[serde(default)]
    quoted_fact: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawFact {
    id: String,
    title: String,
    #[serde(default)]
    primary_genre: Option<String>,
    #[serde(default)]
    genres: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    summary: String,
    claim: String,
    #[serde(default)]
    explanation: Option<String>,
    #[serde(default)]
    sources: Vec<RawSource>,
    status: FactStatus,
    created_at: NaiveDate,
    updated_at: NaiveDate,
    revision: u32,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    duplicate_of: Option<String>,
    #[serde(default)]
    supersedes: Option<String>,
    #[serde(default = "default_canonical")]
    canonical: bool,
    #[serde(default)]
    importance: Option<f32>,
    #[serde(default)]
    editorial: Option<Editorial>,
    #[serde(default)]
    evidence_note: Option<String>,
}

impl FactCollection {
    pub fn facts(&self) -> &[LoadedFact] {
        &self.facts
    }

    pub fn reference_count(&self) -> usize {
        self.by_id.len()
    }
}

pub fn load_taxonomy(root: &Path) -> Result<Taxonomy> {
    load_taxonomy_from(&root.join(TAXONOMY_PATH))
}

pub fn load_taxonomy_from(path: &Path) -> Result<Taxonomy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read taxonomy '{}'", path.display()))?;

    serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse taxonomy '{}'", path.display()))
}

pub fn load_facts(root: &Path) -> Result<FactCollection> {
    load_facts_from(&root.join(FACTS_DIR))
}

pub fn load_facts_from(facts_root: &Path) -> Result<FactCollection> {
    let paths = discover_fact_paths(facts_root)?;

    let mut facts: Vec<LoadedFact> = Vec::new();
    let mut by_id = HashMap::new();
    for path in paths {
        let loaded = load_fact_from(&path)?;
        let index = facts.len();
        if let Some(existing_index) = by_id.insert(loaded.fact.id.clone(), index) {
            let existing_path = facts[existing_index].path.display().to_string();
            bail!(
                "duplicate fact id '{}' found in '{}' and '{}'",
                loaded.fact.id,
                existing_path,
                loaded.path.display()
            );
        }
        facts.push(loaded);
    }

    Ok(FactCollection { facts, by_id })
}

pub fn discover_fact_paths(facts_root: &Path) -> Result<Vec<PathBuf>> {
    if !facts_root.is_dir() {
        bail!("facts directory '{}' does not exist", facts_root.display());
    }

    let mut paths = Vec::new();
    for entry in WalkDir::new(facts_root).sort_by_file_name() {
        let entry = entry.with_context(|| {
            format!("failed to walk facts directory '{}'", facts_root.display())
        })?;

        if entry.file_type().is_file() && is_yaml_file(entry.path()) {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn load_fact_from(path: &Path) -> Result<LoadedFact> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read fact '{}'", path.display()))?;
    let raw_fact: RawFact = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse fact '{}'", path.display()))?;
    let fact = normalize_fact(path, raw_fact)?;

    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .with_context(|| format!("fact path '{}' has no valid file stem", path.display()))?;
    if file_stem != fact.id {
        bail!(
            "fact id '{}' does not match filename '{}'",
            fact.id,
            file_stem
        );
    }

    Ok(LoadedFact {
        path: path.to_path_buf(),
        fact,
    })
}

fn normalize_fact(path: &Path, raw: RawFact) -> Result<Fact> {
    let primary_genre = raw
        .primary_genre
        .as_deref()
        .and_then(normalize_optional_string)
        .map(str::to_owned)
        .or_else(|| infer_primary_genre(path, &raw.id))
        .with_context(|| {
            format!(
                "failed to determine primary_genre for fact '{}'",
                path.display()
            )
        })?;

    let mut genres = raw
        .genres
        .into_iter()
        .filter_map(|genre| normalize_optional_string(&genre).map(str::to_owned))
        .collect::<Vec<_>>();
    if genres.is_empty() {
        genres.push(primary_genre.clone());
    }

    let evidence_note = raw
        .evidence_note
        .as_deref()
        .and_then(normalize_optional_string)
        .map(str::to_owned);
    let sources = normalize_sources(raw.sources, evidence_note);

    Ok(Fact {
        id: raw.id,
        title: raw.title,
        primary_genre,
        genres,
        tags: raw
            .tags
            .into_iter()
            .filter_map(|tag| normalize_optional_string(&tag).map(str::to_owned))
            .collect(),
        summary: raw.summary,
        claim: raw.claim,
        explanation: raw
            .explanation
            .and_then(|value| normalize_optional_string(&value).map(str::to_owned)),
        sources,
        status: raw.status,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
        revision: raw.revision,
        aliases: raw.aliases,
        duplicate_of: raw.duplicate_of,
        supersedes: raw.supersedes,
        canonical: raw.canonical,
        importance: raw.importance,
        editorial: raw.editorial,
    })
}

fn normalize_sources(raw_sources: Vec<RawSource>, evidence_note: Option<String>) -> Vec<Source> {
    raw_sources
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let title = raw
                .title
                .as_deref()
                .and_then(normalize_optional_string)
                .map(str::to_owned)
                .unwrap_or_else(|| fallback_source_title(&raw.publisher, &raw.url));
            let id = raw
                .id
                .as_deref()
                .and_then(normalize_optional_string)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("source-{}", index + 1));
            let quoted_fact = raw
                .quoted_fact
                .as_deref()
                .and_then(normalize_optional_string)
                .map(str::to_owned)
                .or_else(|| (index == 0).then(|| evidence_note.clone()).flatten());

            Source {
                id,
                url: raw.url,
                title,
                publisher: raw.publisher,
                kind: raw.kind,
                accessed_at: raw.accessed_at,
                quoted_fact,
            }
        })
        .collect()
}

fn fallback_source_title(publisher: &str, url: &str) -> String {
    normalize_optional_string(publisher)
        .map(str::to_owned)
        .unwrap_or_else(|| url.to_owned())
}

fn infer_primary_genre(path: &Path, id: &str) -> Option<String> {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .and_then(normalize_optional_string)
        .map(str::to_owned)
        .or_else(|| {
            id.split_once('-')
                .map(|(prefix, _)| prefix)
                .and_then(normalize_optional_string)
                .map(str::to_owned)
        })
}

fn normalize_optional_string(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

const fn default_canonical() -> bool {
    true
}

fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(path)
    }

    #[test]
    fn loads_taxonomy_fixture() {
        let taxonomy =
            load_taxonomy_from(&fixture_path("config/taxonomy.yaml")).expect("taxonomy loads");

        assert_eq!(taxonomy.genres["money"].label, "お金");
        assert_eq!(taxonomy.tags["currency"].label, "貨幣");
    }

    #[test]
    fn recursively_loads_facts_and_builds_reference_index() {
        let facts = load_facts_from(&fixture_path("facts/valid")).expect("facts load");

        assert_eq!(facts.facts().len(), 1);
        assert!(facts.by_id.contains_key("money-001-yen-tree-not-specific"));
    }

    #[test]
    fn rejects_fact_filename_id_mismatch() {
        let err = load_fact_from(&fixture_path(
            "facts/invalid_filename/money/not-the-fact-id.yaml",
        ))
        .expect_err("filename mismatch should fail");

        assert!(
            err.to_string()
                .contains("does not match filename 'not-the-fact-id'")
        );
    }

    #[test]
    fn normalizes_compact_fact_shape() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let facts_dir = temp.path().join("facts/food");
        fs::create_dir_all(&facts_dir).expect("facts dir");
        let fact_path = facts_dir.join("food-0001.yaml");
        fs::write(
            &fact_path,
            r#"
id: food-0001
title: 国産牛は品種名ではない
genres:
  - 食べ物
tags:
  - 農林水産省
summary: summary
claim: claim
sources:
  - url: https://example.com/facts
    publisher: 農林水産省
    kind: official
    accessed_at: 2026-03-15
evidence_note: 公式FAQを参照
status: draft
created_at: 2026-03-15
updated_at: 2026-03-15
revision: 1
"#,
        )
        .expect("write compact fact");

        let loaded = load_fact_from(&fact_path).expect("compact fact loads");

        assert_eq!(loaded.fact.primary_genre, "food");
        assert_eq!(loaded.fact.sources[0].id, "source-1");
        assert_eq!(loaded.fact.sources[0].title, "農林水産省");
        assert_eq!(
            loaded.fact.sources[0].quoted_fact.as_deref(),
            Some("公式FAQを参照")
        );
    }
}
