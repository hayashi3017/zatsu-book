use anyhow::{Context, Result, anyhow, bail};
use chrono::{Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};

use crate::load::{discover_fact_paths, load_taxonomy};

const FACTS_DIR: &str = "facts";
const TEMPLATE_PATH: &str = "templates/fact.yaml";
const MAX_SERIAL: u32 = 9999;

pub fn run(
    root: &Path,
    genre: &str,
    title: &str,
    slug_override: Option<&str>,
    edit: bool,
) -> Result<()> {
    if slug_override.is_some() {
        eprintln!("warning: --slug is ignored; new facts use 4-digit serial ids");
    }
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
    _slug_override: Option<&str>,
    today: NaiveDate,
) -> Result<CreatedFact> {
    let taxonomy = load_taxonomy(root)?;
    if !taxonomy.genres.contains_key(genre) {
        bail!("genre '{}' is not defined in config/taxonomy.yaml", genre);
    }

    let serial = next_serial(root, genre)?;
    let id = format!("{genre}-{serial:04}");
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
        let Some((prefix, serial)) = split_fact_id_prefix_and_serial(stem) else {
            continue;
        };
        if prefix != genre {
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
            "genre '{}' exceeded the 4-digit serial limit (>{MAX_SERIAL})",
            genre
        );
    }
    Ok(next)
}

fn render_template(template: &str, genre: &str, id: &str, title: &str, today: NaiveDate) -> String {
    template
        .replace("<id>", id)
        .replace("<genre-slug>", genre)
        .replace("<serial>", &format_serial_fragment(id))
        .replace("<title>", &yaml_inline_string(title))
        .replace("2026-03-15", &today.to_string())
}

fn format_serial_fragment(id: &str) -> String {
    split_fact_id_prefix_and_serial(id)
        .map(|(_, serial)| serial.to_owned())
        .expect("generated id should have a serial component")
}

fn yaml_inline_string(value: &str) -> String {
    let serialized = serde_yaml::to_string(value).expect("string serialization should succeed");
    let without_doc = serialized.strip_prefix("---\n").unwrap_or(&serialized);
    without_doc
        .strip_suffix('\n')
        .unwrap_or(without_doc)
        .to_owned()
}

fn split_fact_id_prefix_and_serial(id: &str) -> Option<(&str, &str)> {
    let mut parts = id.splitn(3, '-');
    Some((parts.next()?, parts.next()?))
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
            include_str!(
                "../tests/fixtures/facts/valid/money/money-001-yen-tree-not-specific.yaml"
            ),
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

        assert_eq!(created.id, "money-0002");
        let content = fs::read_to_string(&created.path).expect("read created fact");
        assert!(content.contains("id: money-0002"));
        assert!(content.contains("title: 500yen diagonal reeding"));
        assert!(content.contains("primary_genre: money"));
        assert!(content.contains("created_at: 2026-03-15"));
        assert!(content.contains("updated_at: 2026-03-15"));
    }

    #[test]
    fn ignores_explicit_slug_override_for_simple_ids() {
        let temp = temp_repo();

        let created = create_fact(
            temp.path(),
            "money",
            "1円玉の木は特定の木ではない",
            Some("yen-tree-not-specific"),
            NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
        )
        .expect("fact should be created");

        assert_eq!(created.id, "money-0002");
    }
}
