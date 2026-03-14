use clap::{Parser, Subcommand};
use std::process::{self, Command};

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Project automation tasks")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum Commands {
    /// Print a greeting.
    Hello,
    /// Run rustfmt in check mode.
    Fmt,
    /// Run clippy for the workspace.
    Clippy,
    /// Run workspace tests.
    Test,
    /// Run the full local CI sequence.
    Ci,
}

fn main() {
    let cli = Cli::parse();
    if let Err(code) = run(cli.command) {
        process::exit(code);
    }
}

fn run(command: Commands) -> Result<(), i32> {
    match command {
        Commands::Hello => {
            println!("hello");
            Ok(())
        }
        Commands::Fmt => run_cargo(&["fmt", "--check"]),
        Commands::Clippy => run_cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        Commands::Test => run_cargo(&["test", "--workspace"]),
        Commands::Ci => {
            run_cargo(&["fmt", "--check"])?;
            run_cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])?;
            run_cargo(&["test", "--workspace"])
        }
    }
}

fn run_cargo(args: &[&str]) -> Result<(), i32> {
    let status = Command::new("cargo").args(args).status().map_err(|err| {
        eprintln!("failed to run cargo {:?}: {err}", args);
        1
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(status.code().unwrap_or(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hello_subcommand() {
        let cli = Cli::try_parse_from(["xtask", "hello"]).expect("should parse hello");
        assert!(matches!(cli.command, Commands::Hello));
    }

    #[test]
    fn parses_ci_subcommand() {
        let cli = Cli::try_parse_from(["xtask", "ci"]).expect("should parse ci");
        assert!(matches!(cli.command, Commands::Ci));
    }
}
