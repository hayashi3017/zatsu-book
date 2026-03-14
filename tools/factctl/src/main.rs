use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "factctl")]
#[command(about = "Manage fact data and generate mdBook pages")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli.command)
}

fn run(command: Command) -> Result<()> {
    let message = match command {
        Command::New { genre, title } => {
            format!("new is not implemented yet: genre={genre}, title={title}")
        }
        Command::Validate => "validate is not implemented yet".to_owned(),
        Command::BuildPages => "build-pages is not implemented yet".to_owned(),
        Command::Dedupe => "dedupe is not implemented yet".to_owned(),
        Command::Update { id } => format!("update is not implemented yet: id={id}"),
        Command::Stale => "stale is not implemented yet".to_owned(),
        Command::Doctor => "doctor is not implemented yet".to_owned(),
    };

    println!("{message}");
    Ok(())
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
