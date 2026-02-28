//! Terminal search functionality

use regex::{Regex, RegexBuilder};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::{TerminalBuffer, Cell, Result, TerminalError};

/// Search match
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Row (line) of match
    pub row: usize,
    /// Start column
    pub start_col: usize,
    /// End column
    pub end_col: usize,
    /// Match text
    pub text: String,
    /// Match index (for multiple matches)
    pub index: usize,
}

/// Search direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Case sensitive search
    pub case_sensitive: bool,
    /// Use regular expressions
    pub regex: bool,
    /// Match whole words only
    pub whole_word: bool,
    /// Wrap around when reaching end
    pub wrap: bool,
    /// Use fuzzy matching
    pub fuzzy: bool,
    /// Highlight all matches
    pub highlight_all: bool,
    /// Search direction
    pub direction: SearchDirection,
    /// Maximum matches
    pub max_matches: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            regex: false,
            whole_word: false,
            wrap: true,
            fuzzy: false,
            highlight_all: true,
            direction: SearchDirection::Forward,
            max_matches: None,
        }
    }
}

/// Terminal search
pub struct TerminalSearch {
    /// Search query
    query: String,
    /// Search options
    options: SearchOptions,
    /// Current matches
    matches: Vec<SearchMatch>,
    /// Current match index
    current_index: usize,
    /// Fuzzy matcher
    fuzzy_matcher: SkimMatcherV2,
    /// Compiled regex (if regex mode)
    regex: Option<Regex>,
}

impl TerminalSearch {
    /// Create new terminal search
    pub fn new(query: String, options: SearchOptions) -> Result<Self> {
        let regex = if options.regex {
            let mut builder = RegexBuilder::new(&query);
            builder.case_insensitive(!options.case_sensitive);
            Some(builder.build().map_err(|e| 
                TerminalError::SearchError(format!("Invalid regex: {}", e)))?)
        } else {
            None
        };

        Ok(Self {
            query,
            options,
            matches: Vec::new(),
            current_index: 0,
            fuzzy_matcher: SkimMatcherV2::default(),
            regex,
        })
    }

    /// Search in buffer
    pub fn search_buffer(&mut self, buffer: &TerminalBuffer) -> Result<Vec<SearchMatch>> {
        let content = buffer.visible_content();
        let mut matches: Vec<SearchMatch> = Vec::new();
        let mut match_index: usize = 0;

        for (row, line) in content.iter().enumerate() {
            let line_text = line.iter().map(|c| c.character).collect::<String>();
            
            if self.options.fuzzy {
                // Fuzzy matching
                if let Some((_score, indices)) = self.fuzzy_matcher.fuzzy_indices(&line_text, &self.query) {
                    if !indices.is_empty() {
                        let start_col = indices[0];
                        let end_col = *indices.last().unwrap() + 1;
                        
                        matches.push(SearchMatch {
                            row,
                            start_col,
                            end_col,
                            text: self.query.clone(),
                            index: match_index,
                        });
                        match_index += 1;
                    }
                }
            } else if let Some(regex) = &self.regex {
                // Regex matching
                for cap in regex.find_iter(&line_text) {
                    matches.push(SearchMatch {
                        row,
                        start_col: cap.start(),
                        end_col: cap.end(),
                        text: cap.as_str().to_string(),
                        index: match_index,
                    });
                    match_index += 1;
                }
            } else {
                // Plain text matching
                let query = if self.options.case_sensitive {
                    self.query.clone()
                } else {
                    self.query.to_lowercase()
                };
                
                let search_line = if self.options.case_sensitive {
                    line_text.clone()
                } else {
                    line_text.to_lowercase()
                };

                let mut start: usize = 0;
                while let Some(pos) = search_line[start..].find(&query) {
                    let abs_pos: usize = start + pos;
                    
                    if self.options.whole_word {
                        // Check word boundaries
                        let is_word_boundary = |c: char| !c.is_alphanumeric() && c != '_';
                        
                        let prev_char = line_text.chars().nth(abs_pos.saturating_sub(1));
                        let next_char = line_text.chars().nth(abs_pos + query.len());
                        
                        let at_start = abs_pos == 0 || prev_char.map_or(true, is_word_boundary);
                        let at_end = abs_pos + query.len() >= line_text.len() || 
                                    next_char.map_or(true, is_word_boundary);
                        
                        if at_start && at_end {
                            matches.push(SearchMatch {
                                row,
                                start_col: abs_pos,
                                end_col: abs_pos + query.len(),
                                text: line_text[abs_pos..abs_pos + query.len()].to_string(),
                                index: match_index,
                            });
                            match_index += 1;
                        }
                    } else {
                        matches.push(SearchMatch {
                            row,
                            start_col: abs_pos,
                            end_col: abs_pos + query.len(),
                            text: line_text[abs_pos..abs_pos + query.len()].to_string(),
                            index: match_index,
                        });
                        match_index += 1;
                    }
                    
                    start = abs_pos + 1;
                    if start >= line_text.len() {
                        break;
                    }
                }
            }

            if let Some(max) = self.options.max_matches {
                if matches.len() >= max {
                    break;
                }
            }
        }

        self.matches = matches.clone();
        self.current_index = if self.options.direction == SearchDirection::Forward {
            0
        } else {
            self.matches.len().saturating_sub(1)
        };

        Ok(matches)
    }

    /// Get next match
    pub fn next_match(&mut self) -> Option<SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        if self.current_index < self.matches.len() - 1 {
            self.current_index += 1;
        } else if self.options.wrap {
            self.current_index = 0;
        } else {
            return None;
        }

        Some(self.matches[self.current_index].clone())
    }

    /// Get previous match
    pub fn prev_match(&mut self) -> Option<SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        if self.current_index > 0 {
            self.current_index -= 1;
        } else if self.options.wrap {
            self.current_index = self.matches.len() - 1;
        } else {
            return None;
        }

        Some(self.matches[self.current_index].clone())
    }

    /// Get current match
    pub fn current_match(&self) -> Option<SearchMatch> {
        self.matches.get(self.current_index).cloned()
    }

    /// Get all matches
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Get match count
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Get current index
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Clear matches
    pub fn clear(&mut self) {
        self.matches.clear();
        self.current_index = 0;
    }

    /// Update query
    pub fn set_query(&mut self, query: String) -> Result<()> {
        self.query = query;
        
        if self.options.regex {
            let mut builder = RegexBuilder::new(&self.query);
            builder.case_insensitive(!self.options.case_sensitive);
            self.regex = Some(builder.build().map_err(|e| 
                TerminalError::SearchError(format!("Invalid regex: {}", e)))?);
        }
        
        Ok(())
    }

    /// Update options
    pub fn set_options(&mut self, options: SearchOptions) -> Result<()> {
        self.options = options;
        
        if self.options.regex {
            let mut builder = RegexBuilder::new(&self.query);
            builder.case_insensitive(!self.options.case_sensitive);
            self.regex = Some(builder.build().map_err(|e| 
                TerminalError::SearchError(format!("Invalid regex: {}", e)))?);
        } else {
            self.regex = None;
        }
        
        Ok(())
    }
}