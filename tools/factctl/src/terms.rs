use crate::model::Taxonomy;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxonomyKind {
    Genre,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerm {
    pub slug: String,
    pub label: String,
}

pub fn resolve_term(raw: &str, taxonomy: &Taxonomy, kind: TaxonomyKind) -> ResolvedTerm {
    let trimmed = raw.trim();
    if let Some(label) = find_label(trimmed, taxonomy, kind) {
        return ResolvedTerm {
            slug: trimmed.to_owned(),
            label: label.to_owned(),
        };
    }

    if let Some(slug) = find_slug_by_label(trimmed, taxonomy, kind) {
        return ResolvedTerm {
            slug: slug.to_owned(),
            label: trimmed.to_owned(),
        };
    }

    ResolvedTerm {
        slug: fallback_slug(trimmed),
        label: trimmed.to_owned(),
    }
}

pub fn resolve_terms_unique(
    raw_terms: &[String],
    taxonomy: &Taxonomy,
    kind: TaxonomyKind,
) -> Vec<ResolvedTerm> {
    let mut seen = HashSet::new();
    let mut resolved = Vec::new();

    for raw in raw_terms {
        if raw.trim().is_empty() {
            continue;
        }

        let term = resolve_term(raw, taxonomy, kind);
        if seen.insert(term.slug.clone()) {
            resolved.push(term);
        }
    }

    resolved
}

fn find_label<'a>(slug: &str, taxonomy: &'a Taxonomy, kind: TaxonomyKind) -> Option<&'a str> {
    match kind {
        TaxonomyKind::Genre => taxonomy.genres.get(slug),
        TaxonomyKind::Tag => taxonomy.tags.get(slug),
    }
    .map(|entry| entry.label.as_str())
}

fn find_slug_by_label<'a>(
    label: &str,
    taxonomy: &'a Taxonomy,
    kind: TaxonomyKind,
) -> Option<&'a str> {
    let entries = match kind {
        TaxonomyKind::Genre => &taxonomy.genres,
        TaxonomyKind::Tag => &taxonomy.tags,
    };

    entries
        .iter()
        .find_map(|(slug, entry)| (entry.label == label).then_some(slug.as_str()))
}

fn fallback_slug(value: &str) -> String {
    if is_safe_path_segment(value) {
        value.to_owned()
    } else {
        format!("term-{}", stable_hash(value))
    }
}

fn is_safe_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
}

fn stable_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", hash as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Taxonomy, TaxonomyEntry};
    use std::collections::BTreeMap;

    fn sample_taxonomy() -> Taxonomy {
        let genres = [
            (
                "food".to_owned(),
                TaxonomyEntry {
                    label: "食べ物".to_owned(),
                },
            ),
            (
                "local".to_owned(),
                TaxonomyEntry {
                    label: "ご当地".to_owned(),
                },
            ),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let tags = [(
            "currency".to_owned(),
            TaxonomyEntry {
                label: "貨幣".to_owned(),
            },
        )]
        .into_iter()
        .collect::<BTreeMap<_, _>>();

        Taxonomy { genres, tags }
    }

    #[test]
    fn resolves_known_labels_to_slug_paths() {
        let resolved = resolve_term("食べ物", &sample_taxonomy(), TaxonomyKind::Genre);

        assert_eq!(resolved.slug, "food");
        assert_eq!(resolved.label, "食べ物");
    }

    #[test]
    fn preserves_unknown_labels_for_display() {
        let resolved = resolve_term("制度", &sample_taxonomy(), TaxonomyKind::Genre);

        assert_eq!(resolved.slug, "制度");
        assert_eq!(resolved.label, "制度");
    }
}
