use std::io::{self, Write};

use crossterm::{
    ExecutableCommand, cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType},
};

/// Edit multi-line text with raw terminal handling.
/// Returns Some(text) if saved with Ctrl+D, None if cancelled with Esc.
pub fn edit_multiline(prompt: &str, initial: &str) -> io::Result<Option<String>> {
    let mut stdout = io::stdout();

    // Print prompt
    println!("{}", prompt);
    println!("[Enter: newline | Ctrl+D: save | Esc: cancel]");
    println!();

    // Enable raw mode
    terminal::enable_raw_mode()?;

    let result = run_editor(&mut stdout, initial);

    // Always disable raw mode before returning
    terminal::disable_raw_mode()?;

    // Clear line and show result
    println!();

    result
}

fn run_editor(stdout: &mut io::Stdout, initial: &str) -> io::Result<Option<String>> {
    let mut lines: Vec<String> = if initial.is_empty() {
        vec![String::new()]
    } else {
        initial.lines().map(String::from).collect()
    };
    if lines.is_empty() {
        lines.push(String::new());
    }

    let mut cursor_line = lines.len() - 1;
    let mut cursor_col = lines[cursor_line].len();

    // Initial render
    render_editor(stdout, &lines, cursor_line, cursor_col)?;

    loop {
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match (code, modifiers) {
                // Ctrl+D: save and exit
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    return Ok(Some(lines.join("\n")));
                }
                // Esc: cancel
                (KeyCode::Esc, _) => {
                    return Ok(None);
                }
                // Enter: add new line
                (KeyCode::Enter, _) => {
                    let rest = lines[cursor_line].split_off(cursor_col);
                    cursor_line += 1;
                    lines.insert(cursor_line, rest);
                    cursor_col = 0;
                }
                // Backspace
                (KeyCode::Backspace, _) => {
                    if cursor_col > 0 {
                        cursor_col -= 1;
                        lines[cursor_line].remove(cursor_col);
                    } else if cursor_line > 0 {
                        // Join with previous line
                        let current = lines.remove(cursor_line);
                        cursor_line -= 1;
                        cursor_col = lines[cursor_line].len();
                        lines[cursor_line].push_str(&current);
                    }
                }
                // Delete
                (KeyCode::Delete, _) => {
                    if cursor_col < lines[cursor_line].len() {
                        lines[cursor_line].remove(cursor_col);
                    } else if cursor_line < lines.len() - 1 {
                        // Join with next line
                        let next = lines.remove(cursor_line + 1);
                        lines[cursor_line].push_str(&next);
                    }
                }
                // Arrow keys
                (KeyCode::Left, _) => {
                    if cursor_col > 0 {
                        cursor_col -= 1;
                    } else if cursor_line > 0 {
                        cursor_line -= 1;
                        cursor_col = lines[cursor_line].len();
                    }
                }
                (KeyCode::Right, _) => {
                    if cursor_col < lines[cursor_line].len() {
                        cursor_col += 1;
                    } else if cursor_line < lines.len() - 1 {
                        cursor_line += 1;
                        cursor_col = 0;
                    }
                }
                (KeyCode::Up, _) => {
                    if cursor_line > 0 {
                        cursor_line -= 1;
                        cursor_col = cursor_col.min(lines[cursor_line].len());
                    }
                }
                (KeyCode::Down, _) => {
                    if cursor_line < lines.len() - 1 {
                        cursor_line += 1;
                        cursor_col = cursor_col.min(lines[cursor_line].len());
                    }
                }
                // Home/End
                (KeyCode::Home, _) => {
                    cursor_col = 0;
                }
                (KeyCode::End, _) => {
                    cursor_col = lines[cursor_line].len();
                }
                // Regular character input
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    lines[cursor_line].insert(cursor_col, c);
                    cursor_col += 1;
                }
                _ => {}
            }

            render_editor(stdout, &lines, cursor_line, cursor_col)?;
        }
    }
}

fn render_editor(
    stdout: &mut io::Stdout,
    lines: &[String],
    cursor_line: usize,
    cursor_col: usize,
) -> io::Result<()> {
    // Move cursor to start of editing area and clear
    stdout.execute(cursor::MoveToColumn(0))?;

    // Clear all lines we might have rendered
    for _ in 0..lines.len() + 1 {
        stdout.execute(terminal::Clear(ClearType::CurrentLine))?;
        stdout.execute(cursor::MoveDown(1))?;
    }

    // Move back up
    stdout.execute(cursor::MoveUp((lines.len() + 1) as u16))?;

    // Render lines
    for (i, line) in lines.iter().enumerate() {
        stdout.execute(cursor::MoveToColumn(0))?;
        stdout.execute(terminal::Clear(ClearType::CurrentLine))?;
        print!("{}", line);
        if i < lines.len() - 1 {
            println!();
        }
    }

    // Position cursor
    let lines_to_go_up = lines.len() - 1 - cursor_line;
    if lines_to_go_up > 0 {
        stdout.execute(cursor::MoveUp(lines_to_go_up as u16))?;
    }
    stdout.execute(cursor::MoveToColumn(cursor_col as u16))?;

    stdout.flush()?;
    Ok(())
}
