//! Syntax highlighter for a single buffer

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

use super::{
    TokenType, TokenStyle, SyntaxTheme, SyntaxConfig,
    Language, treesitter::TreeSitterHighlighter,
};

/// Highlight token produced by the highlighter
#[derive(Debug, Clone)]
pub struct HighlightToken {
    pub text: String,
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
    pub style: TokenStyle,
}

/// Syntax highlighter for a single buffer
#[derive(Debug, Clone)]
pub struct SyntaxHighlighter {
    /// Buffer ID
    buffer_id: usize,
    /// Language configuration
    language: Option<Language>,
    /// Current theme
    theme: Arc<RwLock<SyntaxTheme>>,
    /// Configuration
    config: Arc<RwLock<SyntaxConfig>>,
    /// Highlight cache
    cache: HighlightCache,
    /// Tree-sitter highlighter (if available)
    ts_highlighter: Option<TreeSitterHighlighter>,
    /// Last highlight time
    last_highlight: Option<DateTime<Utc>>,
}

/// Highlight cache for performance
#[derive(Debug, Clone)]
struct HighlightCache {
    /// Cached lines
    lines: HashMap<usize, Vec<HighlightToken>>,
    /// Version number for invalidation
    version: usize,
    /// Maximum cache size
    max_size: usize,
}

impl HighlightCache {
    fn new(max_size: usize) -> Self {
        Self {
            lines: HashMap::with_capacity(max_size),
            version: 0,
            max_size,
        }
    }

    fn get(&self, line: usize) -> Option<&Vec<HighlightToken>> {
        self.lines.get(&line)
    }

    fn insert(&mut self, line: usize, tokens: Vec<HighlightToken>) {
        if self.lines.len() >= self.max_size {
            // Remove oldest entry
            if let Some(oldest) = self.lines.keys().next().copied() {
                self.lines.remove(&oldest);
            }
        }
        self.lines.insert(line, tokens);
    }

    fn invalidate(&mut self) {
        self.version += 1;
        self.lines.clear();
    }

    fn invalidate_line(&mut self, line: usize) {
        self.lines.remove(&line);
    }
}

impl SyntaxHighlighter {
    /// Create new syntax highlighter
    pub fn new(
        buffer_id: usize,
        language: Option<Language>,
        theme: Arc<RwLock<SyntaxTheme>>,
        config: SyntaxConfig,
    ) -> Self {
        let ts_highlighter = if let Some(lang) = &language {
            if config.enable_tree_sitter {
                TreeSitterHighlighter::new(&lang.clone())
            } else {
                None
            }
        } else {
            None
        };

        Self {
            buffer_id,
            language,
            theme,
            config: Arc::new(RwLock::new(config)),
            cache: HighlightCache::new(1000),
            ts_highlighter,
            last_highlight: None,
        }
    }

    /// Highlight a single line
    pub fn highlight_line(&mut self, line: usize, text: &str) -> Vec<HighlightToken> {
        // Check cache
        if let Some(cached) = self.cache.get(line) {
            return cached.clone();
        }

        // Perform highlighting
        let tokens = if let Some(ts) = &self.ts_highlighter {
            ts.highlight_line(line, text)
        } else if let Some(lang) = &self.language {
            self.highlight_with_regex(line, text, lang)
        } else {
            self.highlight_plain(text)
        };

        // Apply theme colors
        let tokens = self.apply_theme(tokens);

        // Cache result
        self.cache.insert(line, tokens.clone());
        self.last_highlight = Some(Utc::now());

        tokens
    }

    /// Highlight with regex patterns (fallback when Tree-sitter not available)
    fn highlight_with_regex(&self, _line: usize, text: &str, language: &Language) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();

        while pos < chars.len() {
            // Check for comments
            if let Some((start, end)) = self.match_comment(&chars[pos..], language) {
                tokens.push(HighlightToken {
                    text: chars[pos + start..pos + end].iter().collect(),
                    token_type: TokenType::Comment,
                    start: pos + start,
                    end: pos + end,
                    style: TokenStyle::default(),
                });
                pos += end;
                continue;
            }

            // Check for strings
            if let Some((_delim, end)) = self.match_string(&chars[pos..]) {
                tokens.push(HighlightToken {
                    text: chars[pos..pos + end].iter().collect(),
                    token_type: TokenType::String,
                    start: pos,
                    end: pos + end,
                    style: TokenStyle::default(),
                });
                pos += end;
                continue;
            }

            // Check for numbers
            if chars[pos].is_ascii_digit() {
                let start = pos;
                while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '.') {
                    pos += 1;
                }
                tokens.push(HighlightToken {
                    text: chars[start..pos].iter().collect(),
                    token_type: TokenType::Number,
                    start,
                    end: pos,
                    style: TokenStyle::default(),
                });
                continue;
            }

            // Check for keywords
            if chars[pos].is_alphabetic() || chars[pos] == '_' {
                let start = pos;
                while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                    pos += 1;
                }
                let word: String = chars[start..pos].iter().collect();
                let token_type = if language.name == "rust" && Self::is_rust_keyword(&word) {
                    TokenType::Keyword
                } else if language.name == "python" && Self::is_python_keyword(&word) {
                    TokenType::Keyword
                } else {
                    TokenType::Variable
                };
                tokens.push(HighlightToken {
                    text: word,
                    token_type,
                    start,
                    end: pos,
                    style: TokenStyle::default(),
                });
                continue;
            }

            // Default token (punctuation, operators, etc.)
            tokens.push(HighlightToken {
                text: chars[pos].to_string(),
                token_type: TokenType::Punctuation,
                start: pos,
                end: pos + 1,
                style: TokenStyle::default(),
            });
            pos += 1;
        }

        tokens
    }

    /// Highlight plain text (no language)
    fn highlight_plain(&self, text: &str) -> Vec<HighlightToken> {
        vec![HighlightToken {
            text: text.to_string(),
            token_type: TokenType::None,
            start: 0,
            end: text.len(),
            style: TokenStyle::default(),
        }]
    }

    /// Apply theme colors to tokens
    fn apply_theme(&self, tokens: Vec<HighlightToken>) -> Vec<HighlightToken> {
        let theme = self.theme.read();
        
        tokens.into_iter().map(|mut token| {
            if let Some(style) = theme.token_styles.get(&token.token_type.to_string()) {
                token.style = style.clone();
            }
            token
        }).collect()
    }

    /// Match comment at position
    fn match_comment(&self, chars: &[char], language: &Language) -> Option<(usize, usize)> {
        if let Some(line_comment) = &language.comment_syntax.line {
            let comment_chars: Vec<char> = line_comment.chars().collect();
            if chars.starts_with(&comment_chars) {
                return Some((0, chars.len()));
            }
        }

        if let (Some(start), Some(end)) = (&language.comment_syntax.block_start, &language.comment_syntax.block_end) {
            let start_chars: Vec<char> = start.chars().collect();
            let end_chars: Vec<char> = end.chars().collect();

            if chars.starts_with(&start_chars) {
                // Find end of block comment
                for i in 0..chars.len() - end_chars.len() {
                    if chars[i..].starts_with(&end_chars) {
                        return Some((0, i + end_chars.len()));
                    }
                }
                return Some((0, chars.len())); // Unterminated block comment
            }
        }

        None
    }

    /// Match string at position
    fn match_string(&self, chars: &[char]) -> Option<(char, usize)> {
        if chars.is_empty() {
            return None;
        }

        let delimiter = chars[0];
        if delimiter == '"' || delimiter == '\'' {
            let mut pos = 1;
            let mut escaped = false;

            while pos < chars.len() {
                if escaped {
                    escaped = false;
                } else if chars[pos] == '\\' {
                    escaped = true;
                } else if chars[pos] == delimiter {
                    return Some((delimiter, pos + 1));
                }
                pos += 1;
            }
        }

        None
    }

    /// Check if word is Rust keyword
    fn is_rust_keyword(word: &str) -> bool {
        matches!(word,
            "fn" | "let" | "mut" | "if" | "else" | "for" | "while" | "loop" |
            "match" | "pub" | "use" | "mod" | "struct" | "enum" | "impl" |
            "return" | "self" | "Self" | "async" | "await" | "dyn" | "trait"
        )
    }

    /// Check if word is Python keyword
    fn is_python_keyword(word: &str) -> bool {
        matches!(word,
            "def" | "class" | "if" | "elif" | "else" | "for" | "while" |
            "break" | "continue" | "return" | "import" | "from" | "as" |
            "try" | "except" | "finally" | "with" | "lambda" | "yield" |
            "global" | "nonlocal" | "True" | "False" | "None" | "and" |
            "or" | "not" | "is" | "in"
        )
    }

    /// Invalidate cache for a line
    pub fn invalidate_line(&mut self, line: usize) {
        self.cache.invalidate_line(line);
    }

    /// Invalidate entire cache
    pub fn invalidate_all(&mut self) {
        self.cache.invalidate();
    }

    /// Update theme
    pub fn set_theme(&self, theme: Arc<RwLock<SyntaxTheme>>) {
        *self.theme.write() = theme.read().clone();
    }

    /// Update configuration
    pub fn update_config(&self, config: SyntaxConfig) {
        *self.config.write() = config;
    }

    /// Get last highlight time
    pub fn last_highlight(&self) -> Option<DateTime<Utc>> {
        self.last_highlight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_highlighter_creation() {
        let theme = Arc::new(RwLock::new(SyntaxTheme::default_dark()));
        let config = SyntaxConfig::default();
        let highlighter = SyntaxHighlighter::new(0, None, theme, config);
        
        assert!(highlighter.last_highlight().is_none());
    }

    #[test]
    fn test_highlight_plain() {
        let theme = Arc::new(RwLock::new(SyntaxTheme::default_dark()));
        let config = SyntaxConfig::default();
        let mut highlighter = SyntaxHighlighter::new(0, None, theme, config);
        
        let tokens = highlighter.highlight_line(0, "Hello, world!");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::None);
    }
}