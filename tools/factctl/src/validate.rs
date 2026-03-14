use crate::load::{self, LoadedFact};
use crate::model::{Fact, FactStatus, Source, Taxonomy};
use anyhow::{Result, bail};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    location: String,
    message: String,
}

impl ValidationIssue {
    fn repository(message: impl Into<String>) -> Self {
        Self {
            location: "repository".to_owned(),
            message: message.into(),
        }
    }

    fn path(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            location: path.into().display().to_string(),
            message: message.into(),
        }
    }

    fn fact(loaded: &LoadedFact, message: impl Into<String>) -> Self {
        Self::path(loaded.path.clone(), message)
    }

    fn relation(relation: &str, message: impl Into<String>) -> Self {
        Self {
            location: relation.to_owned(),
            message: message.into(),
        }
    }

    pub fn render(&self) -> String {
        format!("{}: {}", self.location, self.message)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub facts: Vec<LoadedFact>,
    pub indexed_ids: usize,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

pub fn run(root: &Path) -> Result<()> {
    let report = validate_repository(root);
    if report.is_valid() {
        println!(
            "validate ok: {} facts ({} indexed ids)",
            report.facts.len(),
            report.indexed_ids
        );
        return Ok(());
    }

    eprintln!("validation failed with {} issue(s):", report.issues.len());
    for issue in &report.issues {
        eprintln!("- {}", issue.render());
    }

    bail!("validation failed")
}

pub fn validate_repository(root: &Path) -> ValidationReport {
    let mut issues = Vec::new();
    let taxonomy = match load::load_taxonomy(root) {
        Ok(taxonomy) => Some(taxonomy),
        Err(err) => {
            issues.push(ValidationIssue::repository(err.to_string()));
            None
        }
    };

    let fact_paths = match load::discover_fact_paths(&root.join("facts")) {
        Ok(paths) => paths,
        Err(err) => {
            issues.push(ValidationIssue::repository(err.to_string()));
            Vec::new()
        }
    };

    let mut facts = Vec::new();
    let mut indexed_ids = 0;
    for path in fact_paths {
        match load::load_fact_from(&path) {
            Ok(loaded) => facts.push(loaded),
            Err(err) => issues.push(ValidationIssue::path(path, err.to_string())),
        }
    }

    if issues.is_empty()
        && let Ok(collection) = load::load_facts(root)
    {
        indexed_ids = collection.reference_count();
        facts = collection.facts().to_vec();
    }

    if indexed_ids == 0 {
        indexed_ids = facts.len();
    }

    if let Some(taxonomy) = taxonomy.as_ref() {
        issues.extend(validate_loaded_facts(&facts, taxonomy));
    }

    issues.sort_by(|left, right| {
        left.location
            .cmp(&right.location)
            .then_with(|| left.message.cmp(&right.message))
    });

    ValidationReport {
        facts,
        indexed_ids,
        issues,
    }
}

fn validate_loaded_facts(facts: &[LoadedFact], taxonomy: &Taxonomy) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut id_paths: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for loaded in facts {
        id_paths
            .entry(loaded.fact.id.clone())
            .or_default()
            .push(loaded.path.clone());
    }

    for (id, paths) in &id_paths {
        if paths.len() > 1 {
            let rendered_paths = paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            issues.push(ValidationIssue::repository(format!(
                "duplicate id '{id}' found in {rendered_paths}"
            )));
        }
    }

    for loaded in facts {
        validate_fact_shape(loaded, taxonomy, &id_paths, &mut issues);
    }

    let unique_facts = facts
        .iter()
        .filter(|loaded| {
            id_paths
                .get(&loaded.fact.id)
                .is_some_and(|paths| paths.len() == 1)
        })
        .collect::<Vec<_>>();
    issues.extend(detect_relation_cycles(
        "duplicate_of",
        &relation_edges(&unique_facts, |fact| fact.duplicate_of.as_deref()),
    ));
    issues.extend(detect_relation_cycles(
        "supersedes",
        &relation_edges(&unique_facts, |fact| fact.supersedes.as_deref()),
    ));

    issues
}

fn validate_fact_shape(
    loaded: &LoadedFact,
    taxonomy: &Taxonomy,
    id_paths: &BTreeMap<String, Vec<PathBuf>>,
    issues: &mut Vec<ValidationIssue>,
) {
    validate_id(loaded, issues);

    let fact = &loaded.fact;
    if !fact.genres.iter().any(|genre| genre == &fact.primary_genre) {
        issues.push(ValidationIssue::fact(
            loaded,
            format!(
                "primary_genre '{}' must be included in genres",
                fact.primary_genre
            ),
        ));
    }

    if !taxonomy.genres.contains_key(&fact.primary_genre) {
        issues.push(ValidationIssue::fact(
            loaded,
            format!(
                "primary_genre '{}' is not defined in taxonomy",
                fact.primary_genre
            ),
        ));
    }

    for genre in &fact.genres {
        if !taxonomy.genres.contains_key(genre) {
            issues.push(ValidationIssue::fact(
                loaded,
                format!("genre '{}' is not defined in taxonomy", genre),
            ));
        }
    }

    for tag in &fact.tags {
        if !taxonomy.tags.contains_key(tag) {
            issues.push(ValidationIssue::fact(
                loaded,
                format!("tag '{}' is not defined in taxonomy", tag),
            ));
        }
    }

    if fact.sources.is_empty() {
        issues.push(ValidationIssue::fact(
            loaded,
            "at least one source is required",
        ));
    }

    if fact.revision < 1 {
        issues.push(ValidationIssue::fact(loaded, "revision must be >= 1"));
    }

    for (index, source) in fact.sources.iter().enumerate() {
        validate_source(loaded, index, source, issues);
    }

    match fact.status {
        FactStatus::Duplicate => {
            if fact.duplicate_of.is_none() {
                issues.push(ValidationIssue::fact(
                    loaded,
                    "status=duplicate requires duplicate_of",
                ));
            }
        }
        _ => {
            if fact.duplicate_of.is_some() {
                issues.push(ValidationIssue::fact(
                    loaded,
                    "duplicate_of is only allowed when status=duplicate",
                ));
            }
        }
    }

    validate_relation_target(
        loaded,
        "duplicate_of",
        fact.duplicate_of.as_deref(),
        id_paths,
        issues,
    );
    validate_relation_target(
        loaded,
        "supersedes",
        fact.supersedes.as_deref(),
        id_paths,
        issues,
    );
}

fn validate_id(loaded: &LoadedFact, issues: &mut Vec<ValidationIssue>) {
    let fact = &loaded.fact;
    let Some((prefix, serial, short_slug)) = split_fact_id(&fact.id) else {
        issues.push(ValidationIssue::fact(
            loaded,
            format!(
                "id '{}' must match <genre-slug>-<serial>-<short-slug>",
                fact.id
            ),
        ));
        return;
    };

    if prefix != fact.primary_genre {
        issues.push(ValidationIssue::fact(
            loaded,
            format!(
                "id '{}' must start with primary_genre '{}'",
                fact.id, fact.primary_genre
            ),
        ));
    }

    if !is_slug(prefix)
        || !is_slug(short_slug)
        || serial.len() != 3
        || !serial.chars().all(|ch| ch.is_ascii_digit())
    {
        issues.push(ValidationIssue::fact(
            loaded,
            format!(
                "id '{}' must match <genre-slug>-<serial>-<short-slug>",
                fact.id
            ),
        ));
    }

    let parent = loaded
        .path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str());
    if let Some(parent) = parent
        && parent != fact.primary_genre
    {
        issues.push(ValidationIssue::fact(
            loaded,
            format!(
                "fact file must live under facts/{}, found facts/{}",
                fact.primary_genre, parent
            ),
        ));
    }
}

fn validate_source(
    loaded: &LoadedFact,
    index: usize,
    source: &Source,
    issues: &mut Vec<ValidationIssue>,
) {
    if Url::parse(&source.url).is_err() {
        issues.push(ValidationIssue::fact(
            loaded,
            format!("source[{index}] has invalid url '{}'", source.url),
        ));
    }
}

fn validate_relation_target(
    loaded: &LoadedFact,
    relation: &str,
    target: Option<&str>,
    id_paths: &BTreeMap<String, Vec<PathBuf>>,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(target) = target else {
        return;
    };

    if target == loaded.fact.id {
        issues.push(ValidationIssue::fact(
            loaded,
            format!("{relation} cannot reference itself"),
        ));
        return;
    }

    match id_paths.get(target).map(Vec::len) {
        Some(1) => {}
        Some(_) => issues.push(ValidationIssue::fact(
            loaded,
            format!("{relation} target '{target}' is duplicated and cannot be resolved"),
        )),
        None => issues.push(ValidationIssue::fact(
            loaded,
            format!("{relation} target '{target}' does not exist"),
        )),
    }
}

fn relation_edges<'a>(
    facts: &[&'a LoadedFact],
    relation: impl Fn(&'a Fact) -> Option<&'a str>,
) -> HashMap<String, String> {
    let unique_ids = facts
        .iter()
        .map(|loaded| loaded.fact.id.as_str())
        .collect::<HashSet<_>>();

    facts
        .iter()
        .filter_map(|loaded| {
            relation(&loaded.fact).and_then(|target| {
                unique_ids
                    .contains(target)
                    .then(|| (loaded.fact.id.clone(), target.to_owned()))
            })
        })
        .collect()
}

fn detect_relation_cycles(relation: &str, edges: &HashMap<String, String>) -> Vec<ValidationIssue> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum VisitState {
        Visiting,
        Done,
    }

    fn visit(
        node: &str,
        relation: &str,
        edges: &HashMap<String, String>,
        states: &mut HashMap<String, VisitState>,
        stack: &mut Vec<String>,
        seen_cycles: &mut HashSet<String>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        states.insert(node.to_owned(), VisitState::Visiting);
        stack.push(node.to_owned());

        if let Some(next) = edges.get(node) {
            match states.get(next).copied() {
                Some(VisitState::Visiting) => {
                    if let Some(start) = stack.iter().position(|id| id == next) {
                        let cycle = stack[start..].to_vec();
                        let mut signature = cycle.clone();
                        signature.sort();
                        let signature = signature.join("|");
                        if seen_cycles.insert(signature) {
                            let mut rendered = cycle;
                            rendered.push(next.clone());
                            issues.push(ValidationIssue::relation(
                                relation,
                                format!("cycle detected: {}", rendered.join(" -> ")),
                            ));
                        }
                    }
                }
                Some(VisitState::Done) => {}
                None => visit(next, relation, edges, states, stack, seen_cycles, issues),
            }
        }

        stack.pop();
        states.insert(node.to_owned(), VisitState::Done);
    }

    let mut issues = Vec::new();
    let mut states = HashMap::new();
    let mut stack = Vec::new();
    let mut seen_cycles = HashSet::new();
    let mut nodes = edges.keys().cloned().collect::<Vec<_>>();
    nodes.sort();

    for node in nodes {
        if !matches!(states.get(&node), Some(VisitState::Done)) {
            visit(
                &node,
                relation,
                edges,
                &mut states,
                &mut stack,
                &mut seen_cycles,
                &mut issues,
            );
        }
    }

    issues
}

fn split_fact_id(id: &str) -> Option<(&str, &str, &str)> {
    let mut parts = id.splitn(3, '-');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Editorial, SourceKind, TaxonomyEntry};
    use chrono::NaiveDate;
    use std::collections::BTreeMap;

    fn sample_taxonomy() -> Taxonomy {
        let mut genres = BTreeMap::new();
        genres.insert(
            "japan".to_owned(),
            TaxonomyEntry {
                label: "日本".to_owned(),
            },
        );
        genres.insert(
            "money".to_owned(),
            TaxonomyEntry {
                label: "お金".to_owned(),
            },
        );

        let mut tags = BTreeMap::new();
        tags.insert(
            "currency".to_owned(),
            TaxonomyEntry {
                label: "貨幣".to_owned(),
            },
        );

        Taxonomy { genres, tags }
    }

    fn sample_loaded_fact(
        id: &str,
        status: FactStatus,
        duplicate_of: Option<&str>,
        supersedes: Option<&str>,
    ) -> LoadedFact {
        LoadedFact {
            path: PathBuf::from(format!("facts/money/{id}.yaml")),
            fact: Fact {
                id: id.to_owned(),
                title: "sample".to_owned(),
                primary_genre: "money".to_owned(),
                genres: vec!["money".to_owned(), "japan".to_owned()],
                tags: vec!["currency".to_owned()],
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
                    quoted_fact: None,
                }],
                status,
                created_at: NaiveDate::from_ymd_opt(2026, 3, 14).expect("valid date"),
                updated_at: NaiveDate::from_ymd_opt(2026, 3, 14).expect("valid date"),
                revision: 1,
                aliases: Vec::new(),
                duplicate_of: duplicate_of.map(str::to_owned),
                supersedes: supersedes.map(str::to_owned),
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
    fn validate_repository_accepts_current_repo() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let report = validate_repository(&root);

        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.facts.len(), 1);
    }

    #[test]
    fn detects_semantic_validation_errors() {
        let taxonomy = sample_taxonomy();
        let mut loaded = sample_loaded_fact(
            "money-001-valid-id",
            FactStatus::Draft,
            Some("missing-id"),
            None,
        );
        loaded.fact.genres = vec!["japan".to_owned()];
        loaded.fact.tags = vec!["unknown-tag".to_owned()];
        loaded.fact.sources.clear();
        loaded.fact.revision = 0;

        let issues = validate_loaded_facts(&[loaded], &taxonomy);
        let rendered = issues
            .iter()
            .map(ValidationIssue::render)
            .collect::<Vec<_>>();

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("primary_genre 'money' must be included in genres"))
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("tag 'unknown-tag' is not defined in taxonomy"))
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("at least one source is required"))
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("revision must be >= 1"))
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("duplicate_of is only allowed when status=duplicate"))
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("duplicate_of target 'missing-id' does not exist"))
        );
    }

    #[test]
    fn detects_duplicate_of_cycles() {
        let taxonomy = sample_taxonomy();
        let first = sample_loaded_fact(
            "money-001-first",
            FactStatus::Duplicate,
            Some("money-002-second"),
            None,
        );
        let second = sample_loaded_fact(
            "money-002-second",
            FactStatus::Duplicate,
            Some("money-001-first"),
            None,
        );

        let issues = validate_loaded_facts(&[first, second], &taxonomy);
        let rendered = issues
            .iter()
            .map(ValidationIssue::render)
            .collect::<Vec<_>>();

        assert!(rendered.iter().any(|line| line.contains(
            "duplicate_of: cycle detected: money-001-first -> money-002-second -> money-001-first"
        )));
    }
}
