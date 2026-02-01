use std::process::ExitCode;

use rustyline::error::ReadlineError;
use rustyline::Editor;

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

    loop {
        match rl.readline("dromos> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);

                match Command::parse(&line) {
                    None => continue, // Empty line
                    Some(Err(e)) => eprintln!("{}", e),
                    Some(Ok(cmd)) => match state.execute(cmd) {
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
