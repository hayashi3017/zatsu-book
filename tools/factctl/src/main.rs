mod cli;
mod load;
mod model;
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
        Command::New { genre, title } => {
            println!("new is not implemented yet: genre={genre}, title={title}");
            Ok(())
        }
        Command::Validate => validate::run(Path::new(".")),
        Command::BuildPages => {
            println!("build-pages is not implemented yet");
            Ok(())
        }
        Command::Dedupe => {
            println!("dedupe is not implemented yet");
            Ok(())
        }
        Command::Update { id } => {
            println!("update is not implemented yet: id={id}");
            Ok(())
        }
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
