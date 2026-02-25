//! Text buffer management using rope data structure

use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use ropey::Rope;
use tokio::fs;
use chrono::{DateTime, Utc};
use super::{Position, Range, EOL};
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Text buffer representing a file or unsaved content
#[derive(Debug, Clone)]
pub struct Buffer {
    /// The actual text content using a rope data structure (efficient for large files)
    content: Rope,
    /// File path if this buffer is associated with a file
    path: Option<PathBuf>,
    /// Whether the buffer has unsaved changes
    modified: bool,
    /// Whether the file was modified externally
    modified_externally: bool,
    /// Line ending style
    eol: EOL,
    /// File encoding
    encoding: String,
    /// Language for syntax highlighting
    language: Option<String>,
    /// Read-only flag
    read_only: bool,
    /// Buffer ID
    id: usize,
    /// Last modified time on disk
    last_modified_disk: Option<DateTime<Utc>>,
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new(path: Option<PathBuf>) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        
        Self {
            content: Rope::new(),
            path,
            modified: false,
            modified_externally: false,
            eol: EOL::default(),
            encoding: "utf-8".to_string(),
            language: None,
            read_only: false,
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            last_modified_disk: None,
        }
    }

    /// Create a buffer from a file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        // Read file content
        let content = fs::read_to_string(path).await?;
        let rope = Rope::from_str(&content);
        
        // Detect line endings
        let eol = if content.contains("\r\n") {
            EOL::CRLF
        } else if content.contains('\r') {
            EOL::CR
        } else {
            EOL::LF
        };
        
        // Detect language from extension
        let language = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_string());
        
        // Get file metadata
        let metadata = fs::metadata(path).await?;
        let last_modified = metadata.modified()
            .map(|t| DateTime::<Utc>::from(t))
            .ok();
        
        Ok(Self {
            content: rope,
            path: Some(path.to_path_buf()),
            modified: false,
            modified_externally: false,
            eol,
            encoding: "utf-8".to_string(),
            language,
            read_only: metadata.permissions().readonly(),
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            last_modified_disk: last_modified,
        })
    }

    /// Save buffer to its associated file
    pub async fn save(&self) -> Result<()> {
        let path = self.path.as_ref()
            .ok_or_else(|| anyhow!("Buffer has no associated file"))?;
        
        self.save_as(path).await
    }

    /// Save buffer to a specific path
    pub async fn save_as<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Write content to file
        let content = self.content.to_string();
        
        // Convert line endings if necessary
        let content = match self.eol {
            EOL::LF => content.replace("\r\n", "\n").replace('\r', "\n"),
            EOL::CRLF => content.replace('\n', "\r\n"),
            EOL::CR => content.replace('\n', "\r"),
        };
        
        fs::write(path, content).await?;
        
        Ok(())
    }

    /// Reload buffer from file
    pub async fn reload(&mut self) -> Result<()> {
        let path = self.path.as_ref()
            .ok_or_else(|| anyhow!("Buffer has no associated file"))?;
        
        let content = fs::read_to_string(path).await?;
        self.content = Rope::from_str(&content);
        self.modified = false;
        self.modified_externally = false;
        
        let metadata = fs::metadata(path).await?;
        self.last_modified_disk = metadata.modified()
            .map(|t| DateTime::<Utc>::from(t))
            .ok();
        
        Ok(())
    }

    /// Insert text at a given character index
    pub fn insert(&mut self, idx: usize, text: &str) {
        self.content.insert(idx, text);
        self.modified = true;
    }

    /// Delete a range of characters
    pub fn delete(&mut self, range: std::ops::Range<usize>) {
        self.content.remove(range);
        self.modified = true;
    }

    /// Get text content as string
    pub fn text(&self) -> String {
        self.content.to_string()
    }

    /// Get line at given index
    pub fn line(&self, idx: usize) -> String {
        if idx < self.content.len_lines() {
            self.content.line(idx).to_string()
        } else {
            String::new()
        }
    }

    /// Get line length (in characters)
    pub fn line_length(&self, idx: usize) -> usize {
        if idx < self.content.len_lines() {
            self.content.line(idx).len_chars()
        } else {
            0
        }
    }

    /// Get character at index
    pub fn char_at(&self, idx: usize) -> Option<char> {
        if idx < self.content.len_chars() {
            Some(self.content.char(idx))
        } else {
            None
        }
    }

    /// Convert line/column position to character index
    pub fn position_to_index(&self, pos: Position) -> usize {
        if pos.line >= self.content.len_lines() {
            return self.content.len_chars();
        }
        
        let line_start = self.content.line_to_char(pos.line);
        let line = self.content.line(pos.line);
        
        // Clamp column to line length
        let max_col = line.len_chars();
        let col = pos.column.min(max_col);
        
        line_start + col
    }

    /// Convert character index to line/column position
    pub fn index_to_position(&self, idx: usize) -> Position {
        let line = self.content.char_to_line(idx);
        let line_start = self.content.line_to_char(line);
        let column = idx - line_start;
        
        Position { line, column }
    }

    /// Get total number of lines
    pub fn line_count(&self) -> usize {
        self.content.len_lines()
    }

    /// Get total number of characters
    pub fn len(&self) -> usize {
        self.content.len_chars()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.content.len_chars() == 0
    }

    /// Get size in bytes
    pub fn bytes(&self) -> usize {
        self.content.len_bytes()
    }

    /// Count words in buffer
    pub fn word_count(&self) -> usize {
        self.text()
            .split_whitespace()
            .count()
    }

    /// Clear buffer content
    pub fn clear(&mut self) {
        self.content = Rope::new();
        self.modified = true;
    }

    /// Get buffer path
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Check if buffer is modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Set modified externally flag
    pub fn set_modified_externally(&mut self, modified: bool) {
        self.modified_externally = modified;
    }

    /// Check if modified externally
    pub fn is_modified_externally(&self) -> bool {
        self.modified_externally
    }

    /// Get line ending style
    pub fn eol(&self) -> EOL {
        self.eol
    }

    /// Set line ending style
    pub fn set_eol(&mut self, eol: EOL) {
        self.eol = eol;
        self.modified = true;
    }

    /// Get language
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Set language
    pub fn set_language(&mut self, language: Option<String>) {
        self.language = language;
    }

    /// Check if buffer is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Get buffer ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Search for text in buffer
    pub fn search(&self, query: &str, case_sensitive: bool) -> Vec<Range> {
        let mut matches = Vec::new();
        let content = self.text();
        
        let search_fn: Box<dyn Fn(&str) -> Vec<usize>> = if case_sensitive {
            Box::new(|s: &str| s.match_indices(query).map(|(i, _)| i).collect())
        } else {
            let lower_query = query.to_lowercase();
            Box::new(move |s: &str| {
                s.to_lowercase()
                    .match_indices(&lower_query)
                    .map(|(i, _)| i)
                    .collect()
            })
        };
        
        for (line_idx, line) in content.lines().enumerate() {
            for match_idx in search_fn(line) {
                let start = Position {
                    line: line_idx,
                    column: match_idx,
                };
                let end = Position {
                    line: line_idx,
                    column: match_idx + query.len(),
                };
                matches.push(Range::new(start, end));
            }
        }
        
        matches
    }

    /// Replace all occurrences
    pub fn replace_all(&mut self, query: &str, replace: &str, case_sensitive: bool) -> usize {
        let matches = self.search(query, case_sensitive);
        let count = matches.len();
        
        if count > 0 {
            // Replace from end to start to maintain positions
            for range in matches.into_iter().rev() {
                let start = self.position_to_index(range.start);
                let end = self.position_to_index(range.end);
                
                self.delete(start..end);
                self.insert(start, replace);
            }
        }
        
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_new_buffer() {
        let buffer = Buffer::new(None);
        assert!(buffer.is_empty());
        assert!(!buffer.is_modified());
        assert!(buffer.path().is_none());
    }

    #[test]
fn test_insert_and_delete() {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "Hello");
        assert_eq!(buffer.text(), "Hello");
        assert!(buffer.is_modified());
        
        buffer.delete(1..3);
        assert_eq!(buffer.text(), "Hlo");
    }

    #[test]
    fn test_position_conversion() {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "Hello\nWorld");
        
        let pos = Position { line: 1, column: 2 };
        let idx = buffer.position_to_index(pos);
        assert_eq!(idx, 7); // "Hello\nWo".len()
        
        let pos2 = buffer.index_to_position(7);
        assert_eq!(pos2.line, 1);
        assert_eq!(pos2.column, 2);
    }

    #[tokio::test]
    async fn test_save_and_load() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_path_buf();
        
        let mut buffer = Buffer::new(Some(path.clone()));
        buffer.insert(0, "Test content");
        buffer.save().await?;
        
        let loaded = Buffer::from_file(path).await?;
        assert_eq!(loaded.text(), "Test content");
        
        Ok(())
    }

    #[test]
    fn test_search() {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "Hello world\nHello rust");
        
        let matches = buffer.search("Hello", true);
        assert_eq!(matches.len(), 2);
        
        let matches = buffer.search("hello", false);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_replace_all() {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "Hello world, hello rust");
        
        let count = buffer.replace_all("hello", "hi", false);
        assert_eq!(count, 2);
        assert_eq!(buffer.text(), "hi world, hi rust");
    }
}