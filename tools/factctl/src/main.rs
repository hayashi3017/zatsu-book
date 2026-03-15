mod cli;
mod dedupe;
mod load;
mod model;
mod new;
mod normalize;
mod render;
mod update;
mod validate;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use std::path::Path;

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli.command)
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::New {
            genre,
            title,
            slug,
            edit,
        } => new::run(Path::new("."), &genre, &title, slug.as_deref(), edit),
        Command::Validate => validate::run(Path::new(".")),
        Command::BuildPages => render::build_pages(Path::new(".")),
        Command::Dedupe {
            fail_on_high_confidence_duplicate,
        } => dedupe::run(Path::new("."), fail_on_high_confidence_duplicate),
        Command::Update { id, edit } => update::run(Path::new("."), &id, edit),
        Command::Stale => {
            println!("stale is not implemented yet");
            Ok(())
        }
        Command::Doctor => {
            println!("doctor is not implemented yet");
            Ok(())
        }
    }
}
