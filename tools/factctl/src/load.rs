use crate::model::{Fact, Taxonomy};
use anyhow::{Context, Result, bail};
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

pub fn load_fact_from(path: &Path) -> Result<LoadedFact> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read fact '{}'", path.display()))?;
    let fact: Fact = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse fact '{}'", path.display()))?;

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
}
