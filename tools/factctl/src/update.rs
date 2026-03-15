use anyhow::{Context, Result, anyhow, bail};
use chrono::{Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};

use crate::load::{discover_fact_paths, load_fact_from};

const FACTS_DIR: &str = "facts";

pub fn run(root: &Path, id: &str, edit: bool) -> Result<()> {
    let updated = update_fact(root, id, Local::now().date_naive())?;
    println!(
        "updated {} (revision {}, updated_at {})",
        updated.path.display(),
        updated.revision,
        updated.updated_at
    );

    if edit {
        open_in_editor(&updated.path)?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdatedFact {
    path: PathBuf,
    revision: u32,
    updated_at: NaiveDate,
}

fn update_fact(root: &Path, id: &str, today: NaiveDate) -> Result<UpdatedFact> {
    let path = find_fact_path(root, id)?;
    let loaded = load_fact_from(&path)?;
    let revision = loaded.fact.revision + 1;
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read fact '{}'", path.display()))?;
    let rewritten = rewrite_metadata(&raw, revision, today)?;
    fs::write(&path, rewritten)
        .with_context(|| format!("failed to write fact '{}'", path.display()))?;

    Ok(UpdatedFact {
        path,
        revision,
        updated_at: today,
    })
}

fn find_fact_path(root: &Path, id: &str) -> Result<PathBuf> {
    let facts_root = root.join(FACTS_DIR);
    if !facts_root.exists() {
        bail!("facts directory '{}' does not exist", facts_root.display());
    }

    let matches = discover_fact_paths(&facts_root)?
        .into_iter()
        .filter(|path| path.file_stem().and_then(|stem| stem.to_str()) == Some(id))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(anyhow!("fact '{}' was not found", id)),
        _ => Err(anyhow!("fact '{}' is duplicated across multiple files", id)),
    }
}

fn rewrite_metadata(raw: &str, revision: u32, today: NaiveDate) -> Result<String> {
    let mut saw_revision = false;
    let mut saw_updated_at = false;
    let mut rendered = String::with_capacity(raw.len());

    for line in raw.split_inclusive('\n') {
        let newline = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let body = line.strip_suffix(newline).unwrap_or(line);

        if !body.starts_with(' ') && !body.starts_with('\t') && body.starts_with("updated_at:") {
            rendered.push_str(&format!("updated_at: {today}{newline}"));
            saw_updated_at = true;
        } else if !body.starts_with(' ') && !body.starts_with('\t') && body.starts_with("revision:")
        {
            rendered.push_str(&format!("revision: {revision}{newline}"));
            saw_revision = true;
        } else {
            rendered.push_str(line);
        }
    }

    if !saw_updated_at {
        bail!("fact YAML is missing a top-level updated_at field");
    }
    if !saw_revision {
        bail!("fact YAML is missing a top-level revision field");
    }

    Ok(rendered)
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
        fs::create_dir_all(temp.path().join("facts/money")).expect("facts dir");
        fs::write(
            temp.path()
                .join("facts/money/money-001-yen-tree-not-specific.yaml"),
            include_str!("../../../facts/money/money-001-yen-tree-not-specific.yaml"),
        )
        .expect("seed fact");
        temp
    }

    #[test]
    fn updates_revision_and_updated_at_in_place() {
        let temp = temp_repo();

        let updated = update_fact(
            temp.path(),
            "money-001-yen-tree-not-specific",
            NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
        )
        .expect("fact should be updated");

        assert_eq!(updated.revision, 2);
        let content = fs::read_to_string(updated.path).expect("read updated fact");
        assert!(content.contains("updated_at: 2026-03-15"));
        assert!(content.contains("revision: 2"));
        assert!(content.contains("created_at: 2026-03-14"));
    }

    #[test]
    fn errors_when_target_fact_does_not_exist() {
        let temp = temp_repo();
        let err = update_fact(
            temp.path(),
            "money-999-missing",
            NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
        )
        .expect_err("missing fact should fail");

        assert!(err.to_string().contains("was not found"));
    }
}
