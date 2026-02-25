//! Syntax highlighting module for Parsec IDE
//!
//! Provides syntax highlighting using Tree-sitter for accurate,
//! incremental parsing with support for multiple languages.

mod highlighter;
mod treesitter;
mod highlight_style;

pub use highlighter::*;
pub use treesitter::*;
pub use highlight_style::*;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Main syntax system managing languages and highlighters
pub struct SyntaxSystem {
    /// Available languages
    languages: Arc<RwLock<HashMap<String, Language>>>,
    /// Active theme
    theme: Arc<RwLock<SyntaxTheme>>,
    /// Highlighters for each buffer
    highlighters: Arc<RwLock<HashMap<usize, SyntaxHighlighter>>>,
    /// Configuration
    config: SyntaxConfig,
}

/// Language definition
#[derive(Debug, Clone)]
pub struct Language {
    pub name: String,
    pub extensions: Vec<String>,
    pub tree_sitter_language: Option<tree_sitter::Language>,
    pub highlights_query: Option<String>,
    pub injections_query: Option<String>,
    pub indentation_rules: IndentationRules,
    pub comment_syntax: CommentSyntax,
}

/// Indentation rules for a language
#[derive(Debug, Clone)]
pub struct IndentationRules {
    pub increase: Vec<String>,
    pub decrease: Vec<String>,
    pub ignore: Vec<String>,
}

/// Comment syntax for a language
#[derive(Debug, Clone)]
pub struct CommentSyntax {
    pub line: Option<String>,
    pub block_start: Option<String>,
    pub block_end: Option<String>,
}

/// Syntax configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxConfig {
    pub enable_highlighting: bool,
    pub enable_tree_sitter: bool,
    pub highlight_delay_ms: u64,
    pub max_line_length: usize,
    pub rainbow_brackets: bool,
    pub bracket_pair_colorization: bool,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            enable_highlighting: true,
            enable_tree_sitter: true,
            highlight_delay_ms: 100,
            max_line_length: 10000,
            rainbow_brackets: true,
            bracket_pair_colorization: true,
        }
    }
}

/// Syntax theme with colors and styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxTheme {
    pub name: String,
    pub theme_type: ThemeType,
    pub colors: HashMap<String, String>,
    pub token_styles: HashMap<String, TokenStyle>,
}

/// Theme type (dark/light)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeType {
    Dark,
    Light,
    HighContrast,
}

/// Style for a token type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStyle {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Default for TokenStyle {
    fn default() -> Self {
        Self {
            foreground: None,
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

/// Token type for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    Keyword,
    Function,
    Type,
    Variable,
    Constant,
    String,
    Comment,
    Number,
    Operator,
    Punctuation,
    Bracket,
    Annotation,
    Preprocessor,
    Whitespace,
    Error,
    None,
}

impl TokenType {
    pub fn to_string(&self) -> String {
        match self {
            TokenType::Keyword => "keyword".to_string(),
            TokenType::Function => "function".to_string(),
            TokenType::Type => "type".to_string(),
            TokenType::Variable => "variable".to_string(),
            TokenType::Constant => "constant".to_string(),
            TokenType::String => "string".to_string(),
            TokenType::Comment => "comment".to_string(),
            TokenType::Number => "number".to_string(),
            TokenType::Operator => "operator".to_string(),
            TokenType::Punctuation => "punctuation".to_string(),
            TokenType::Bracket => "bracket".to_string(),
            TokenType::Annotation => "annotation".to_string(),
            TokenType::Preprocessor => "preprocessor".to_string(),
            TokenType::Whitespace => "whitespace".to_string(),
            TokenType::Error => "error".to_string(),
            TokenType::None => "none".to_string(),
        }
    }
}

impl SyntaxSystem {
    /// Create new syntax system
    pub fn new(config: SyntaxConfig) -> Self {
        let mut system = Self {
            languages: Arc::new(RwLock::new(HashMap::new())),
            theme: Arc::new(RwLock::new(SyntaxTheme::default_dark())),
            highlighters: Arc::new(RwLock::new(HashMap::new())),
            config,
        };
        
        // Load built-in languages
        system.load_builtin_languages();
        
        system
    }

    /// Load built-in languages
    fn load_builtin_languages(&mut self) {
        let languages = vec![
            Language::rust(),
            Language::python(),
            Language::javascript(),
            Language::typescript(),
            Language::html(),
            Language::css(),
            Language::json(),
            Language::markdown(),
            Language::toml(),
            Language::yaml(),
        ];
        
        let mut map = self.languages.write();
        for lang in languages {
            map.insert(lang.name.clone(), lang);
        }
    }

    /// Get language for file
    pub fn language_for_file(&self, path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?;
        self.languages.read()
            .values()
            .find(|lang| lang.extensions.contains(&ext.to_string()))
            .cloned()
    }

    /// Create highlighter for buffer
    pub fn create_highlighter(&self, buffer_id: usize, language: Option<Language>) -> SyntaxHighlighter {
    let highlighter = SyntaxHighlighter::new(
        buffer_id,
        language,
        self.theme.clone(),
        self.config.clone(),
    );
    
    // Clone before inserting so we can return the original
    self.highlighters.write().insert(buffer_id, highlighter.clone());
    highlighter
}


    /// Get highlighter for buffer
    pub fn highlighter(&self, buffer_id: usize) -> Option<SyntaxHighlighter> {
        self.highlighters.read().get(&buffer_id).cloned()
    }

    /// Remove highlighter for buffer
    pub fn remove_highlighter(&self, buffer_id: usize) {
        self.highlighters.write().remove(&buffer_id);
    }

    /// Set theme
    pub fn set_theme(&self, theme: SyntaxTheme) {
        *self.theme.write() = theme;
        
        // Update all highlighters
        for highlighter in self.highlighters.read().values() {
            highlighter.set_theme(self.theme.clone());
        }
    }

    /// Get current theme
    pub fn theme(&self) -> SyntaxTheme {
        self.theme.read().clone()
    }

    /// Get all available languages
    pub fn languages(&self) -> Vec<String> {
        self.languages.read().keys().cloned().collect()
    }

    /// Add custom language
    pub fn add_language(&self, language: Language) {
        self.languages.write().insert(language.name.clone(), language);
    }
}

impl SyntaxTheme {
    /// Default dark theme
    pub fn default_dark() -> Self {
        let mut colors = HashMap::new();
        colors.insert("background".to_string(), "#1e1e1e".to_string());
        colors.insert("foreground".to_string(), "#d4d4d4".to_string());
        colors.insert("selection".to_string(), "#264f78".to_string());
        colors.insert("line_number".to_string(), "#858585".to_string());
        
        let mut token_styles = HashMap::new();
        token_styles.insert("keyword".to_string(), TokenStyle {
            foreground: Some("#569cd6".to_string()),
            background: None,
            bold: true,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("string".to_string(), TokenStyle {
            foreground: Some("#ce9178".to_string()),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("comment".to_string(), TokenStyle {
            foreground: Some("#6a9955".to_string()),
            background: None,
            bold: false,
            italic: true,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("function".to_string(), TokenStyle {
            foreground: Some("#dcdcaa".to_string()),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("type".to_string(), TokenStyle {
            foreground: Some("#4ec9b0".to_string()),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("number".to_string(), TokenStyle {
            foreground: Some("#b5cea8".to_string()),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("operator".to_string(), TokenStyle {
            foreground: Some("#d4d4d4".to_string()),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        Self {
            name: "Default Dark".to_string(),
            theme_type: ThemeType::Dark,
            colors,
            token_styles,
        }
    }

    /// Default light theme
    pub fn default_light() -> Self {
        let mut colors = HashMap::new();
        colors.insert("background".to_string(), "#ffffff".to_string());
        colors.insert("foreground".to_string(), "#000000".to_string());
        colors.insert("selection".to_string(), "#add6ff".to_string());
        colors.insert("line_number".to_string(), "#969696".to_string());
        
        let mut token_styles = HashMap::new();
        token_styles.insert("keyword".to_string(), TokenStyle {
            foreground: Some("#0000ff".to_string()),
            background: None,
            bold: true,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("string".to_string(), TokenStyle {
            foreground: Some("#a31515".to_string()),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        });
        
        token_styles.insert("comment".to_string(), TokenStyle {
            foreground: Some("#008000".to_string()),
            background: None,
            bold: false,
            italic: true,
            underline: false,
            strikethrough: false,
        });
        
        Self {
            name: "Default Light".to_string(),
            theme_type: ThemeType::Light,
            colors,
            token_styles,
        }
    }
}

impl Language {
    /// Create Rust language
    pub fn rust() -> Self {
        Self {
            name: "rust".to_string(),
            extensions: vec!["rs".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r"\{[^}]*$".to_string(),
                    r"\([^)]*$".to_string(),
                    r"\[[^\]]*$".to_string(),
                ],
                decrease: vec![
                    r"^\s*\}".to_string(),
                    r"^\s*\)".to_string(),
                    r"^\s*\]".to_string(),
                ],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: Some("//".to_string()),
                block_start: Some("/*".to_string()),
                block_end: Some("*/".to_string()),
            },
        }
    }

    /// Create Python language
    pub fn python() -> Self {
        Self {
            name: "python".to_string(),
            extensions: vec!["py".to_string(), "py3".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r":\s*$".to_string(),
                ],
                decrease: vec![
                    r"^\s*(return|pass|break|continue|raise)\b".to_string(),
                ],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: Some("#".to_string()),
                block_start: Some("'''".to_string()),
                block_end: Some("'''".to_string()),
            },
        }
    }

    /// Create JavaScript language
    pub fn javascript() -> Self {
        Self {
            name: "javascript".to_string(),
            extensions: vec!["js".to_string(), "jsx".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r"\{[^}]*$".to_string(),
                    r"\([^)]*$".to_string(),
                ],
                decrease: vec![
                    r"^\s*\}".to_string(),
                    r"^\s*\)".to_string(),
                ],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: Some("//".to_string()),
                block_start: Some("/*".to_string()),
                block_end: Some("*/".to_string()),
            },
        }
    }

    /// Create TypeScript language
    pub fn typescript() -> Self {
        let mut js = Self::javascript();
        js.name = "typescript".to_string();
        js.extensions = vec!["ts".to_string(), "tsx".to_string()];
        js
    }

    /// Create HTML language
    pub fn html() -> Self {
        Self {
            name: "html".to_string(),
            extensions: vec!["html".to_string(), "htm".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r"<\w+[^>]*>[^<]*$".to_string(),
                ],
                decrease: vec![
                    r"</\w+>".to_string(),
                ],
                ignore: vec![
                    r"<[^>]*/>".to_string(),
                ],
            },
            comment_syntax: CommentSyntax {
                line: None,
                block_start: Some("<!--".to_string()),
                block_end: Some("-->".to_string()),
            },
        }
    }

    /// Create CSS language
    pub fn css() -> Self {
        Self {
            name: "css".to_string(),
            extensions: vec!["css".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r"\{[^}]*$".to_string(),
                ],
                decrease: vec![
                    r"\}".to_string(),
                ],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: None,
                block_start: Some("/*".to_string()),
                block_end: Some("*/".to_string()),
            },
        }
    }

    /// Create JSON language
    pub fn json() -> Self {
        Self {
            name: "json".to_string(),
            extensions: vec!["json".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r"\{[^}]*$".to_string(),
                    r"\[[^\]]*$".to_string(),
                ],
                decrease: vec![
                    r"\}".to_string(),
                    r"\]".to_string(),
                ],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: None,
                block_start: None,
                block_end: None,
            },
        }
    }

    /// Create Markdown language
    pub fn markdown() -> Self {
        Self {
            name: "markdown".to_string(),
            extensions: vec!["md".to_string(), "markdown".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![],
                decrease: vec![],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: None,
                block_start: None,
                block_end: None,
            },
        }
    }

    /// Create TOML language
    pub fn toml() -> Self {
        Self {
            name: "toml".to_string(),
            extensions: vec!["toml".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r"\[[^\]]*$".to_string(),
                ],
                decrease: vec![
                    r"\]".to_string(),
                ],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: Some("#".to_string()),
                block_start: None,
                block_end: None,
            },
        }
    }

    /// Create YAML language
    pub fn yaml() -> Self {
        Self {
            name: "yaml".to_string(),
            extensions: vec!["yaml".to_string(), "yml".to_string()],
            tree_sitter_language: None,
            highlights_query: None,
            injections_query: None,
            indentation_rules: IndentationRules {
                increase: vec![
                    r":\s*$".to_string(),
                ],
                decrease: vec![],
                ignore: vec![],
            },
            comment_syntax: CommentSyntax {
                line: Some("#".to_string()),
                block_start: None,
                block_end: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_system_creation() {
        let system = SyntaxSystem::new(SyntaxConfig::default());
        assert!(!system.languages().is_empty());
    }

    #[test]
    fn test_language_detection() {
        let system = SyntaxSystem::new(SyntaxConfig::default());
        let path = Path::new("test.rs");
        let lang = system.language_for_file(path);
        assert!(lang.is_some());
        assert_eq!(lang.unwrap().name, "rust");
    }

    #[test]
    fn test_theme_switching() {
        let system = SyntaxSystem::new(SyntaxConfig::default());
        let dark = SyntaxTheme::default_dark();
        let light = SyntaxTheme::default_light();
        
        system.set_theme(dark);
        assert_eq!(system.theme().name, "Default Dark");
        
        system.set_theme(light);
        assert_eq!(system.theme().name, "Default Light");
    }
}