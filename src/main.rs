use std::process::ExitCode;

use git_version::git_version;
use rustyline::error::ReadlineError;
use rustyline::Editor;

const VERSION: &str = git_version!(
    args = ["--tags", "--always", "--dirty=-modified"],
    fallback = env!("CARGO_PKG_VERSION")
);
const BUILD_TIME: &str = env!("BUILD_TIMESTAMP");

use dromos::cli::{Command, DromosHelper, ReplState};
use dromos::config::StorageConfig;

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
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

    println!(r"     _                               ");
    println!(r"  __| |_ __ ___  _ __ ___   ___  ___ ");
    println!(r" / _` | '__/ _ \| '_ ` _ \ / _ \/ __|");
    println!(r"| (_| | | | (_) | | | | | | (_) \__ \  {VERSION}");
    println!(r" \__,_|_|  \___/|_| |_| |_|\___/|___/  {BUILD_TIME}");
    println!(r"");
    println!("  - type a command, e.g. \"help\" or \"exit\"");
    println!("  - press tab for autocomplete, and up/down for history");

    loop {
        match rl.readline("\ncmd> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);

                match Command::parse(&line) {
                    None => continue, // Empty line
                    Some(Err(e)) => eprintln!("{}", e),
                    Some(Ok(cmd)) => match state.execute(cmd, &mut rl) {
                        Ok(true) => {} // Continue
                        Ok(false) => break, // Quit requested
                        Err(e) => eprintln!("Error: {}", e),
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
                eprintln!("Error: {}", e);
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
    directories::ProjectDirs::from("", "", "dromos")
        .map(|dirs| dirs.data_dir().join("history.txt"))
}
