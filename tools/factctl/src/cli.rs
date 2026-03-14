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
    },
    /// Validate fact data.
    Validate,
    /// Generate mdBook input pages.
    BuildPages,
    /// Detect duplicate candidates.
    Dedupe,
    /// Update an existing fact.
    Update { id: String },
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
        ])
        .expect("new should parse");

        assert!(matches!(cli.command, Command::New { .. }));
    }
}
