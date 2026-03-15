use crate::{dedupe, render, stale, validate};
use anyhow::{Result, bail};
use std::path::Path;

pub fn run(root: &Path) -> Result<()> {
    let mut failures = Vec::new();

    run_step("validate", &mut failures, || validate::run(root));
    run_step("dedupe", &mut failures, || dedupe::run(root, false));
    run_step("stale", &mut failures, || stale::run(root));
    run_step("build-pages(dry-run)", &mut failures, || {
        render::build_pages_dry_run(root)
    });

    if failures.is_empty() {
        println!("doctor complete: all checks passed");
        Ok(())
    } else {
        bail!(
            "doctor failed: {} step(s) failed: {}",
            failures.len(),
            failures.join(", ")
        )
    }
}

fn run_step(label: &str, failures: &mut Vec<String>, f: impl FnOnce() -> Result<()>) {
    println!("doctor: {label}");
    if let Err(err) = f() {
        eprintln!("doctor step failed [{label}]: {err:#}");
        failures.push(label.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
            include_str!("../../../facts/money/money-001-yen-tree-not-specific.yaml"),
        )
        .expect("seed fact");
        temp
    }

    #[test]
    fn doctor_runs_reports_without_writing_pages() {
        let temp = temp_repo();

        run(temp.path()).expect("doctor should succeed");

        assert!(
            temp.path()
                .join("generated/reports/duplicate_candidates.md")
                .exists()
        );
        assert!(
            temp.path()
                .join("generated/reports/stale_sources.md")
                .exists()
        );
        assert!(!temp.path().join("src/README.md").exists());
    }
}
