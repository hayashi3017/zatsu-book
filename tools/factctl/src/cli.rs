use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "factctl")]
#[command(about = "Manage fact data and generate mdBook pages")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate a new fact template.
    New {
        #[arg(long)]
        genre: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        edit: bool,
    },
    /// Validate fact data.
    Validate,
    /// Generate mdBook input pages.
    BuildPages,
    /// Detect duplicate candidates.
    Dedupe {
        #[arg(long = "fail-on-high-confidence-duplicate")]
        fail_on_high_confidence_duplicate: bool,
    },
    /// Update an existing fact.
    Update {
        id: String,
        #[arg(long)]
        edit: bool,
    },
    /// Report stale sources.
    Stale,
    /// Run the aggregated project checks.
    Doctor,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_validate_subcommand() {
        let cli = Cli::try_parse_from(["factctl", "validate"]).expect("validate should parse");
        assert!(matches!(cli.command, Command::Validate));
    }

    #[test]
    fn parses_new_subcommand() {
        let cli = Cli::try_parse_from([
            "factctl",
            "new",
            "--genre",
            "money",
            "--title",
            "1円玉の木は特定の木ではない",
            "--slug",
            "yen-tree-not-specific",
            "--edit",
        ])
        .expect("new should parse");

        assert!(matches!(
            cli.command,
            Command::New {
                slug: Some(_),
                edit: true,
                ..
            }
        ));
    }

    #[test]
    fn parses_update_subcommand() {
        let cli = Cli::try_parse_from([
            "factctl",
            "update",
            "money-001-yen-tree-not-specific",
            "--edit",
        ])
        .expect("update should parse");

        assert!(matches!(cli.command, Command::Update { edit: true, .. }));
    }

    #[test]
    fn parses_dedupe_subcommand() {
        let cli = Cli::try_parse_from(["factctl", "dedupe", "--fail-on-high-confidence-duplicate"])
            .expect("dedupe should parse");

        assert!(matches!(
            cli.command,
            Command::Dedupe {
                fail_on_high_confidence_duplicate: true
            }
        ));
    }

    #[test]
    fn parses_stale_subcommand() {
        let cli = Cli::try_parse_from(["factctl", "stale"]).expect("stale should parse");
        assert!(matches!(cli.command, Command::Stale));
    }

    #[test]
    fn parses_doctor_subcommand() {
        let cli = Cli::try_parse_from(["factctl", "doctor"]).expect("doctor should parse");
        assert!(matches!(cli.command, Command::Doctor));
    }
}
