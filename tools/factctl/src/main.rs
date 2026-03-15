mod cli;
mod load;
mod model;
mod new;
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
        Command::Dedupe => {
            println!("dedupe is not implemented yet");
            Ok(())
        }
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
