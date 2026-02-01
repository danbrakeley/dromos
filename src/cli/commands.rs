use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    Add { file: PathBuf },
    Build { source: PathBuf, target: String },
    Link { files: Vec<PathBuf> },
    Links { target: String },
    List,
    Rm { target: String },
    Search { query: String },
    Hash { file: PathBuf },
    Help,
    Quit,
}

impl Command {
    /// Parse a command line into a Command.
    /// Returns None if the line is empty or only whitespace.
    /// Returns Some(Err) if the command is invalid.
    pub fn parse(line: &str) -> Option<Result<Command, String>> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let parts = parse_quoted_args(line);
        if parts.is_empty() {
            return None;
        }

        let cmd = parts[0].to_lowercase();
        let args = &parts[1..];

        Some(match cmd.as_str() {
            "add" => {
                if args.is_empty() {
                    Err("Usage: add <file>".to_string())
                } else {
                    Ok(Command::Add {
                        file: PathBuf::from(&args[0]),
                    })
                }
            }
            "build" => {
                if args.len() < 2 {
                    Err("Usage: build <source_file> <target_hash>".to_string())
                } else {
                    Ok(Command::Build {
                        source: PathBuf::from(&args[0]),
                        target: args[1].clone(),
                    })
                }
            }
            "link" => {
                if args.is_empty() {
                    Err("Usage: link <file1> [file2]".to_string())
                } else {
                    Ok(Command::Link {
                        files: args.iter().map(PathBuf::from).collect(),
                    })
                }
            }
            "links" => {
                if args.is_empty() {
                    Err("Usage: links <file|hash>".to_string())
                } else {
                    Ok(Command::Links {
                        target: args[0].clone(),
                    })
                }
            }
            "list" | "ls" => Ok(Command::List),
            "rm" | "remove" => {
                if args.is_empty() {
                    Err("Usage: rm <hash>".to_string())
                } else {
                    Ok(Command::Rm {
                        target: args[0].clone(),
                    })
                }
            }
            "search" => {
                if args.is_empty() {
                    Err("Usage: search <query>".to_string())
                } else {
                    Ok(Command::Search {
                        query: args.join(" "),
                    })
                }
            }
            "hash" => {
                if args.is_empty() {
                    Err("Usage: hash <file>".to_string())
                } else {
                    Ok(Command::Hash {
                        file: PathBuf::from(&args[0]),
                    })
                }
            }
            "help" | "?" => Ok(Command::Help),
            "quit" | "exit" => Ok(Command::Quit),
            _ => Err(format!("Unknown command: {}", cmd)),
        })
    }
}

/// Parse a command line respecting quoted strings.
/// Handles both single and double quotes.
fn parse_quoted_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for c in line.chars() {
        match (c, in_quote) {
            // Start of quoted string
            ('"' | '\'', None) => {
                in_quote = Some(c);
            }
            // End of quoted string
            (q, Some(quote)) if q == quote => {
                in_quote = None;
            }
            // Space outside quotes - end of argument
            (' ', None) => {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            }
            // Any other character
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quoted_args() {
        assert_eq!(
            parse_quoted_args(r#"add "foo bar.nes""#),
            vec!["add", "foo bar.nes"]
        );
        assert_eq!(
            parse_quoted_args("add 'foo bar.nes'"),
            vec!["add", "foo bar.nes"]
        );
        assert_eq!(
            parse_quoted_args("link a.nes b.nes"),
            vec!["link", "a.nes", "b.nes"]
        );
    }

    #[test]
    fn test_parse_commands() {
        assert!(matches!(
            Command::parse("add test.nes"),
            Some(Ok(Command::Add { .. }))
        ));
        assert!(matches!(Command::parse("list"), Some(Ok(Command::List))));
        assert!(matches!(Command::parse("ls"), Some(Ok(Command::List))));
        assert!(matches!(
            Command::parse("rm abc123"),
            Some(Ok(Command::Rm { target })) if target == "abc123"
        ));
        assert!(matches!(
            Command::parse("remove abc123"),
            Some(Ok(Command::Rm { target })) if target == "abc123"
        ));
        assert!(matches!(Command::parse("rm"), Some(Err(_))));
        assert!(matches!(Command::parse("quit"), Some(Ok(Command::Quit))));
        assert!(matches!(Command::parse("exit"), Some(Ok(Command::Quit))));
        assert!(matches!(Command::parse(""), None));
        assert!(matches!(Command::parse("   "), None));
    }
}
