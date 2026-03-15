use anyhow::{Context, Result, anyhow, bail};
use chrono::{Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

use crate::load::{discover_fact_paths, load_taxonomy};

const FACTS_DIR: &str = "facts";
const TEMPLATE_PATH: &str = "templates/fact.yaml";
const MAX_SERIAL: u32 = 999;

pub fn run(
    root: &Path,
    genre: &str,
    title: &str,
    slug_override: Option<&str>,
    edit: bool,
) -> Result<()> {
    let created = create_fact(root, genre, title, slug_override, Local::now().date_naive())?;
    println!("created {} ({})", created.path.display(), created.id);

    if edit {
        open_in_editor(&created.path)?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreatedFact {
    id: String,
    path: PathBuf,
}

fn create_fact(
    root: &Path,
    genre: &str,
    title: &str,
    slug_override: Option<&str>,
    today: NaiveDate,
) -> Result<CreatedFact> {
    let taxonomy = load_taxonomy(root)?;
    if !taxonomy.genres.contains_key(genre) {
        bail!("genre '{}' is not defined in config/taxonomy.yaml", genre);
    }

    let short_slug = slug_override
        .map(validate_slug_override)
        .transpose()?
        .unwrap_or_else(|| slugify_title(title));
    let serial = next_serial(root, genre)?;
    let id = format!("{genre}-{serial:03}-{short_slug}");
    let facts_dir = root.join(FACTS_DIR).join(genre);
    let path = facts_dir.join(format!("{id}.yaml"));
    if path.exists() {
        bail!("fact file '{}' already exists", path.display());
    }

    let template = fs::read_to_string(root.join(TEMPLATE_PATH)).with_context(|| {
        format!(
            "failed to read template '{}'",
            root.join(TEMPLATE_PATH).display()
        )
    })?;
    let content = render_template(&template, genre, &id, title, today);

    fs::create_dir_all(&facts_dir)
        .with_context(|| format!("failed to create '{}'", facts_dir.display()))?;
    fs::write(&path, content)
        .with_context(|| format!("failed to write fact '{}'", path.display()))?;

    Ok(CreatedFact { id, path })
}

fn validate_slug_override(slug: &str) -> Result<String> {
    if is_slug(slug) {
        Ok(slug.to_owned())
    } else {
        Err(anyhow!(
            "slug '{}' must contain only lowercase ascii letters, digits, and hyphens",
            slug
        ))
    }
}

fn next_serial(root: &Path, genre: &str) -> Result<u32> {
    let facts_root = root.join(FACTS_DIR);
    if !facts_root.exists() {
        return Ok(1);
    }

    let mut max_serial = 0_u32;
    for path in discover_fact_paths(&facts_root)? {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let Some((prefix, serial, _)) = split_fact_id(stem) else {
            continue;
        };
        if prefix != genre || serial.len() != 3 {
            continue;
        }
        let Ok(parsed) = serial.parse::<u32>() else {
            continue;
        };
        max_serial = max_serial.max(parsed);
    }

    let next = max_serial + 1;
    if next > MAX_SERIAL {
        bail!(
            "genre '{}' exceeded the 3-digit serial limit (>{MAX_SERIAL})",
            genre
        );
    }
    Ok(next)
}

fn render_template(template: &str, genre: &str, id: &str, title: &str, today: NaiveDate) -> String {
    template
        .replace("<genre-slug>", genre)
        .replace("<serial>", &format_serial_fragment(id))
        .replace("<short-slug>", &format_short_slug_fragment(id))
        .replace("<title>", &yaml_inline_string(title))
        .replace("2026-03-15", &today.to_string())
}

fn format_serial_fragment(id: &str) -> String {
    split_fact_id(id)
        .map(|(_, serial, _)| serial.to_owned())
        .expect("generated id should have a serial component")
}

fn format_short_slug_fragment(id: &str) -> String {
    split_fact_id(id)
        .map(|(_, _, slug)| slug.to_owned())
        .expect("generated id should have a slug component")
}

fn yaml_inline_string(value: &str) -> String {
    let serialized = serde_yaml::to_string(value).expect("string serialization should succeed");
    let without_doc = serialized.strip_prefix("---\n").unwrap_or(&serialized);
    without_doc
        .strip_suffix('\n')
        .unwrap_or(without_doc)
        .to_owned()
}

fn slugify_title(title: &str) -> String {
    let normalized = title.nfkc().collect::<String>();
    let mut slug = String::new();
    let mut needs_separator = false;
    let mut has_letter = false;

    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            let lower = ch.to_ascii_lowercase();
            if !slug.is_empty() && needs_separator {
                slug.push('-');
            }
            slug.push(lower);
            has_letter |= lower.is_ascii_lowercase();
            needs_separator = false;
        } else if ch.to_digit(10).is_some() {
            if !slug.is_empty() && needs_separator {
                slug.push('-');
            }
            slug.push(
                char::from_digit(ch.to_digit(10).expect("checked digit"), 10).expect("ascii digit"),
            );
            needs_separator = false;
        } else if !slug.is_empty() {
            needs_separator = true;
        }
    }

    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        return format!("fact-{}", stable_title_hash(title));
    }

    if has_letter {
        slug
    } else {
        format!("{slug}-{}", stable_title_hash(title))
    }
}

fn stable_title_hash(title: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in title.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", hash as u32)
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

fn open_in_editor(path: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| anyhow!("--edit requires VISUAL or EDITOR to be set"))?;

    let args = shlex::split(&editor).unwrap_or_else(|| vec![editor.clone()]);
    let (program, rest) = args
        .split_first()
        .ok_or_else(|| anyhow!("failed to parse editor command"))?;

    let status = std::process::Command::new(program)
        .args(rest)
        .arg(path)
        .status()
        .with_context(|| format!("failed to launch editor '{}'", editor))?;
    if !status.success() {
        bail!("editor '{}' exited with status {}", editor, status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_repo() -> TempDir {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("config")).expect("config dir");
        fs::create_dir_all(temp.path().join("templates")).expect("template dir");
        fs::create_dir_all(temp.path().join("facts/money")).expect("facts dir");
        fs::write(
            temp.path().join("config/taxonomy.yaml"),
            "genres:\n  money:\n    label: お金\ntags:\n  currency:\n    label: 貨幣\n",
        )
        .expect("taxonomy");
        fs::write(
            temp.path().join("templates/fact.yaml"),
            include_str!("../../../templates/fact.yaml"),
        )
        .expect("template");
        fs::write(
            temp.path()
                .join("facts/money/money-001-yen-tree-not-specific.yaml"),
            include_str!("../../../facts/money/money-001-yen-tree-not-specific.yaml"),
        )
        .expect("seed fact");
        temp
    }

    #[test]
    fn creates_new_fact_from_template() {
        let temp = temp_repo();

        let created = create_fact(
            temp.path(),
            "money",
            "500yen diagonal reeding",
            None,
            NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
        )
        .expect("fact should be created");

        assert_eq!(created.id, "money-002-500yen-diagonal-reeding");
        let content = fs::read_to_string(&created.path).expect("read created fact");
        assert!(content.contains("id: money-002-500yen-diagonal-reeding"));
        assert!(content.contains("title: 500yen diagonal reeding"));
        assert!(content.contains("primary_genre: money"));
        assert!(content.contains("created_at: 2026-03-15"));
        assert!(content.contains("updated_at: 2026-03-15"));
    }

    #[test]
    fn uses_hash_fallback_for_non_ascii_titles() {
        assert_eq!(slugify_title("1円玉の木は特定の木ではない"), "1-e546c134");
    }

    #[test]
    fn accepts_explicit_slug_override() {
        let temp = temp_repo();

        let created = create_fact(
            temp.path(),
            "money",
            "1円玉の木は特定の木ではない",
            Some("yen-tree-not-specific"),
            NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
        )
        .expect("fact should be created");

        assert_eq!(created.id, "money-002-yen-tree-not-specific");
    }
}
