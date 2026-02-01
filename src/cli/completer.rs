use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

/// Helper for rustyline that provides command and filename completion.
pub struct DromosHelper {
    file_completer: FilenameCompleter,
}

impl DromosHelper {
    pub fn new() -> Self {
        Self {
            file_completer: FilenameCompleter::new(),
        }
    }
}

impl Default for DromosHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl Helper for DromosHelper {}
impl Hinter for DromosHelper {
    type Hint = String;
}
impl Highlighter for DromosHelper {}
impl Validator for DromosHelper {}

/// Commands that accept file path arguments.
const FILE_COMMANDS: &[&str] = &["add", "link", "hash"];

/// All available commands.
const ALL_COMMANDS: &[&str] = &[
    "add", "link", "list", "ls", "search", "hash", "help", "quit", "exit",
];

impl Completer for DromosHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let line_to_cursor = &line[..pos];
        let parts: Vec<&str> = line_to_cursor.split_whitespace().collect();

        // No input yet or at start of line - complete commands
        if parts.is_empty() {
            return Ok((0, command_completions("")));
        }

        let cmd = parts[0].to_lowercase();

        // Still typing the first word (command) - complete commands
        if parts.len() == 1 && !line_to_cursor.ends_with(' ') {
            return Ok((0, command_completions(&cmd)));
        }

        // After command - check if it takes file arguments
        if FILE_COMMANDS.contains(&cmd.as_str()) {
            return self.file_completer.complete(line, pos, ctx);
        }

        // No completions for other commands (search takes free text, list/help/quit take nothing)
        Ok((pos, vec![]))
    }
}

/// Return command completions matching the given prefix.
fn command_completions(prefix: &str) -> Vec<Pair> {
    ALL_COMMANDS
        .iter()
        .filter(|c| c.starts_with(prefix))
        .map(|c| Pair {
            display: c.to_string(),
            replacement: c.to_string(),
        })
        .collect()
}
