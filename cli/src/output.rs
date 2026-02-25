//! Output formatting for CLI
//!
//! Provides colored and structured output for the command line.

use std::io::Write;

use colored::*;
use serde::Serialize;
use serde_json;

use crate::args::OutputFormat;

/// Output formatter for CLI commands
pub struct OutputFormatter {
    format: OutputFormat,
    color: bool,
    quiet: bool,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(format: OutputFormat, color: bool, quiet: bool) -> Self {
        Self {
            format,
            color,
            quiet,
        }
    }

    /// Print a value in the configured format
    pub fn print<T: Serialize + std::fmt::Debug>(&self, value: &T) -> anyhow::Result<()> {
        if self.quiet {
            return Ok(());
        }

        match self.format {
            OutputFormat::Json => self.print_json(value),
            OutputFormat::Yaml => self.print_yaml(value),
            OutputFormat::Table => self.print_table(value),
            _ => self.print_text(value),
        }
    }

    /// Print success message
    pub fn success(&self, message: &str) {
        if self.quiet {
            return;
        }
        if self.color {
            println!("{}", message.green());
        } else {
            println!("{}", message);
        }
    }

    /// Print error message
    pub fn error(&self, message: &str) {
        if self.quiet && self.format != OutputFormat::Quiet {
            return;
        }
        if self.color {
            eprintln!("{}", message.red());
        } else {
            eprintln!("{}", message);
        }
    }

    /// Print warning message
    pub fn warning(&self, message: &str) {
        if self.quiet {
            return;
        }
        if self.color {
            println!("{}", message.yellow());
        } else {
            println!("{}", message);
        }
    }

    /// Print info message
    pub fn info(&self, message: &str) {
        if self.quiet {
            return;
        }
        if self.color {
            println!("{}", message.cyan());
        } else {
            println!("{}", message);
        }
    }

    /// Print debug message
    pub fn debug(&self, message: &str) {
        if self.quiet {
            return;
        }
        if self.color {
            println!("{}", message.dimmed());
        } else {
            println!("{}", message);
        }
    }

    /// Print a table from structured data
    pub fn print_table<T: Serialize + std::fmt::Debug>(&self, data: &T) -> anyhow::Result<()> {
        // Simplified table printing - in production, use a proper table formatter
        println!("{:#?}", data);
        Ok(())
    }

    /// Print JSON output
    fn print_json<T: Serialize>(&self, value: &T) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(value)?;
        println!("{}", json);
        Ok(())
    }

    /// Print YAML output
    fn print_yaml<T: Serialize>(&self, value: &T) -> anyhow::Result<()> {
        #[cfg(feature = "yaml")]
        {
            let yaml = serde_yaml::to_string(value)?;
            println!("{}", yaml);
        }
        #[cfg(not(feature = "yaml"))]
        {
            // Fallback to JSON
            self.print_json(value)?;
        }
        Ok(())
    }

    /// Print text output
    fn print_text<T: std::fmt::Debug>(&self, value: &T) -> anyhow::Result<()> {
        println!("{:?}", value);
        Ok(())
    }

    /// Create a spinner for long-running operations
    pub fn spinner(&self, message: &str) -> Option<Spinner> {
        if self.quiet {
            return None;
        }
        Some(Spinner::new(message, self.color))
    }

    /// Create a progress bar
    pub fn progress_bar(&self, total: u64, message: &str) -> Option<ProgressBar> {
        if self.quiet {
            return None;
        }
        Some(ProgressBar::new(total, message, self.color))
    }

    /// Confirm an action with the user
    pub fn confirm(&self, prompt: &str) -> bool {
        if self.quiet {
            return false;
        }
        println!("{} (y/N): ", prompt);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        input.trim().to_lowercase() == "y"
    }

    /// Prompt for input
    pub fn prompt(&self, prompt: &str) -> Option<String> {
        if self.quiet {
            return None;
        }
        println!("{}: ", prompt);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok()?;
        Some(input.trim().to_string())
    }

    /// Prompt for password (hidden input)
    pub fn prompt_password(&self, prompt: &str) -> Option<String> {
        if self.quiet {
            return None;
        }
        rpassword::prompt_password(format!("{}: ", prompt)).ok()
    }
}

/// Spinner for showing progress
pub struct Spinner {
    message: String,
    frames: Vec<&'static str>,
    current: usize,
    color: bool,
}

impl Spinner {
    pub fn new(message: &str, color: bool) -> Self {
        Self {
            message: message.to_string(),
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current: 0,
            color,
        }
    }

    pub fn tick(&mut self) {
        print!("\r{} {} ", self.frames[self.current], self.message);
        self.current = (self.current + 1) % self.frames.len();
        std::io::stdout().flush().unwrap();
    }

    pub fn finish(&self, message: &str) {
        if self.color {
            println!("\r✅ {} {}", message.green(), self.message);
        } else {
            println!("\r✅ {} {}", message, self.message);
        }
    }

    pub fn fail(&self, message: &str) {
        if self.color {
            println!("\r❌ {} {}", message.red(), self.message);
        } else {
            println!("\r❌ {} {}", message, self.message);
        }
    }
}

/// Progress bar for showing progress
pub struct ProgressBar {
    total: u64,
    current: u64,
    message: String,
    width: usize,
    color: bool,
}

impl ProgressBar {
    pub fn new(total: u64, message: &str, color: bool) -> Self {
        Self {
            total,
            current: 0,
            message: message.to_string(),
            width: 50,
            color,
        }
    }

    pub fn set(&mut self, current: u64) {
        self.current = current.min(self.total);
        self.render();
    }

    pub fn inc(&mut self, delta: u64) {
        self.current = (self.current + delta).min(self.total);
        self.render();
    }

    pub fn finish(&self) {
        println!();
    }

    fn render(&self) {
        let percent = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            0
        };
        let filled = (self.width * percent) / 100;
        let empty = self.width - filled;

        let bar = if self.color {
            format!(
                "{}{}",
                "█".repeat(filled).green().to_string(),
                "░".repeat(empty).dimmed().to_string()
            )
        } else {
            format!("{}{}", "█".repeat(filled), "░".repeat(empty))
        };

        print!(
            "\r{} [{}] {}/{} ({:3}%) ",
            self.message, bar, self.current, self.total, percent
        );
        std::io::stdout().flush().unwrap();
    }
}

/// Table builder for structured output
pub struct TableBuilder {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    widths: Vec<usize>,
}

impl TableBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            widths: Vec::new(),
        }
    }

    pub fn headers(mut self, headers: Vec<&str>) -> Self {
        self.headers = headers.iter().map(|h| h.to_string()).collect();
        self.widths = headers.iter().map(|h| h.len()).collect();
        self
    }

    pub fn add_row(mut self, row: Vec<&str>) -> Self {
        let row_str: Vec<String> = row.iter().map(|r| r.to_string()).collect();
        for (i, cell) in row_str.iter().enumerate() {
            if i < self.widths.len() {
                self.widths[i] = self.widths[i].max(cell.len());
            }
        }
        self.rows.push(row_str);
        self
    }

    pub fn build(&self) -> String {
        let mut output = String::new();

        if !self.headers.is_empty() {
            // Header
            for (i, header) in self.headers.iter().enumerate() {
                output.push_str(&format!("{:width$}  ", header, width = self.widths[i]));
            }
            output.push('\n');

            // Separator
            for width in &self.widths {
                output.push_str(&format!("{:-<width$}  ", "", width = *width));
            }
            output.push('\n');
        }

        // Rows
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < self.widths.len() {
                    output.push_str(&format!("{:width$}  ", cell, width = self.widths[i]));
                }
            }
            output.push('\n');
        }

        output
    }

    pub fn print(&self) {
        print!("{}", self.build());
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!("text".parse(), Ok(OutputFormat::Text));
        assert_eq!("json".parse(), Ok(OutputFormat::Json));
        assert_eq!("table".parse(), Ok(OutputFormat::Table));
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_table_builder() {
        let table = TableBuilder::new()
            .headers(vec!["Name", "Version", "Publisher"])
            .add_row(vec!["rust-analyzer", "1.0.0", "rust-lang"])
            .add_row(vec!["python", "2.0.0", "ms-python"]);

        let output = table.build();
        assert!(output.contains("rust-analyzer"));
        assert!(output.contains("python"));
    }
}

// Convenience free functions used by command handlers
pub fn print_table<T: Serialize + std::fmt::Debug>(data: T, format: OutputFormat) -> anyhow::Result<()> {
    let formatter = OutputFormatter::new(format, true, format == OutputFormat::Quiet);
    formatter.print_table(&data)
}

pub fn print_json<T: Serialize>(data: T, format: OutputFormat) -> anyhow::Result<()> {
    let formatter = OutputFormatter::new(format, true, format == OutputFormat::Quiet);
    formatter.print_json(&data)
}

pub fn print_success(message: &str) {
    let formatter = OutputFormatter::new(OutputFormat::Text, true, false);
    formatter.success(message)
}

pub fn print_error(message: &str) {
    let formatter = OutputFormatter::new(OutputFormat::Text, true, false);
    formatter.error(message)
}