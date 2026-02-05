use std::process::ExitCode;

use git_version::git_version;
use rustyline::Editor;
use rustyline::error::ReadlineError;

const VERSION: &str = git_version!(
    args = ["--tags", "--always", "--dirty=-modified"],
    fallback = env!("CARGO_PKG_VERSION")
);
const BUILD_TIME: &str = env!("BUILD_TIMESTAMP");

use dromos::cli::{Command, DromosHelper, ReplState, theme};
use dromos::config::StorageConfig;

fn main() -> ExitCode {
    theme::init();

    if let Err(e) = run() {
        eprintln!("{} {}", theme::error("Error:"), e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run() -> dromos::Result<()> {
    let config = StorageConfig::default_paths().ok_or_else(|| {
        dromos::DromosError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine data directory",
        ))
    })?;

    let mut state = ReplState::new(config)?;
    let mut rl = Editor::new().expect("Failed to initialize readline");
    rl.set_helper(Some(DromosHelper::new()));

    // Try to load history (ignore errors)
    let history_path = dirs_history_path();
    if let Some(path) = &history_path {
        let _ = rl.load_history(path);
    }

    theme::print_banner(VERSION, BUILD_TIME);
    println!();
    println!("  - type a command, e.g. \"help\" or \"exit\"");
    println!("  - press tab for autocomplete, and up/down for history");

    let prompt_str = format!("\n{}> ", theme::prompt("dromos"));

    loop {
        match rl.readline(&prompt_str) {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);

                match Command::parse(&line) {
                    None => continue, // Empty line
                    Some(Err(e)) => eprintln!("{}", theme::error(&e)),
                    Some(Ok(cmd)) => match state.execute(cmd, &mut rl) {
                        Ok(true) => {}      // Continue
                        Ok(false) => break, // Quit requested
                        Err(e) => eprintln!("{} {}", theme::error("Error:"), e),
                    },
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("{} {}", theme::error("Error:"), e);
                break;
            }
        }
    }

    // Save history
    if let Some(path) = &history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn dirs_history_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "dromos").map(|dirs| dirs.data_dir().join("history.txt"))
}
