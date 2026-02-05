//! Color theming for CLI output.
//!
//! Respects `NO_COLOR` environment variable and TTY detection.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::style::Stylize;

/// Global flag for whether colors are enabled.
static COLORS_ENABLED: AtomicBool = AtomicBool::new(false);

/// Initialize color support detection.
/// Call this once at startup before any themed output.
pub fn init() {
    let enabled = std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal();
    COLORS_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Check if colors are currently enabled.
fn colors_enabled() -> bool {
    COLORS_ENABLED.load(Ordering::Relaxed)
}

// ─── Semantic Functions ─────────────────────────────────────────────────────

/// Format text as an error (red).
pub fn error(text: &str) -> String {
    if colors_enabled() {
        text.red().to_string()
    } else {
        text.to_string()
    }
}

/// Format text as a warning (yellow).
pub fn warning(text: &str) -> String {
    if colors_enabled() {
        text.yellow().to_string()
    } else {
        text.to_string()
    }
}

/// Format text as success (green).
pub fn success(text: &str) -> String {
    if colors_enabled() {
        text.green().to_string()
    } else {
        text.to_string()
    }
}

/// Format text as info (cyan).
pub fn info(text: &str) -> String {
    if colors_enabled() {
        text.cyan().to_string()
    } else {
        text.to_string()
    }
}

// ─── Data Display Functions ────────────────────────────────────────────────

/// Format a title (bright white).
pub fn title(text: &str) -> String {
    if colors_enabled() {
        text.white().to_string()
    } else {
        text.to_string()
    }
}

/// Format a categorical label like ROM type (yellow).
pub fn label(text: &str) -> String {
    if colors_enabled() {
        text.yellow().to_string()
    } else {
        text.to_string()
    }
}

/// Format secondary metadata like version or link count (cyan).
pub fn meta(text: &str) -> String {
    if colors_enabled() {
        text.cyan().to_string()
    } else {
        text.to_string()
    }
}

// ─── Chrome Functions ───────────────────────────────────────────────────────

/// Format text as a prompt (bright blue, bold).
pub fn prompt(text: &str) -> String {
    if colors_enabled() {
        text.blue().bold().to_string()
    } else {
        text.to_string()
    }
}

/// Format text as dim/secondary (dark grey).
pub fn dim(text: &str) -> String {
    if colors_enabled() {
        text.dark_grey().to_string()
    } else {
        text.to_string()
    }
}

/// Format text as a header (bold white).
pub fn header(text: &str) -> String {
    if colors_enabled() {
        text.bold().to_string()
    } else {
        text.to_string()
    }
}

// ─── Banner Functions ──────────────────────────────────────────────────────

/// Format the ASCII logo (bright blue).
pub fn logo(text: &str) -> String {
    if colors_enabled() {
        text.blue().to_string()
    } else {
        text.to_string()
    }
}

/// Format the build version in banner (dark green).
pub fn build_version(text: &str) -> String {
    if colors_enabled() {
        text.dark_green().to_string()
    } else {
        text.to_string()
    }
}

/// Format the build date in banner (dark red).
pub fn build_date(text: &str) -> String {
    if colors_enabled() {
        text.dark_red().to_string()
    } else {
        text.to_string()
    }
}

const LOGO: [&str; 5] = [
    r"     _                               ",
    r"  __| |_ __ ___  _ __ ___   ___  ___ ",
    r" / _` | '__/ _ \| '_ ` _ \ / _ \/ __|",
    r"| (_| | | | (_) | | | | | | (_) \__ \",
    r" \__,_|_|  \___/|_| |_| |_|\___/|___/",
];

/// Print the startup banner with version and build time.
pub fn print_banner(version: &str, build_time: &str) {
    println!("{}", logo(LOGO[0]));
    println!("{}", logo(LOGO[1]));
    println!("{}", logo(LOGO[2]));
    println!("{}  {}", logo(LOGO[3]), build_version(version));
    println!("{}  {}", logo(LOGO[4]), build_date(build_time));
}

// ─── Helper Functions ───────────────────────────────────────────────────────

/// Format a hash with a styled suffix ("...").
/// Takes the short hash prefix (e.g., first 16 chars) and appends green "...".
pub fn styled_hash(short_hash: &str) -> String {
    if colors_enabled() {
        format!("{}{}", short_hash.blue(), "...".dark_blue())
    } else {
        format!("{}...", short_hash)
    }
}
