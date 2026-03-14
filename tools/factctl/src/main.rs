mod cli;
mod load;
mod model;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use std::path::Path;

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli.command)
}

fn run(command: Command) -> Result<()> {
    let message = match command {
        Command::New { genre, title } => {
            format!("new is not implemented yet: genre={genre}, title={title}")
        }
        Command::Validate => {
            let taxonomy = load::load_taxonomy(Path::new("."))?;
            let facts = load::load_facts(Path::new("."))?;
            format!(
                "loaded {} facts, {} indexed ids, {} genres, {} tags",
                facts.facts().len(),
                facts.reference_count(),
                taxonomy.genres.len(),
                taxonomy.tags.len()
            )
        }
        Command::BuildPages => "build-pages is not implemented yet".to_owned(),
        Command::Dedupe => "dedupe is not implemented yet".to_owned(),
        Command::Update { id } => format!("update is not implemented yet: id={id}"),
        Command::Stale => "stale is not implemented yet".to_owned(),
        Command::Doctor => "doctor is not implemented yet".to_owned(),
    };

    println!("{message}");
    Ok(())
}
