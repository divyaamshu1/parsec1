//! Tree-sitter integration for accurate syntax highlighting

#![allow(unexpected_cfgs)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use anyhow::{Result, anyhow};
use tree_sitter::{Parser, Query, Tree, Node, Language as TsLanguage};
use tracing::{debug, warn};

use super::{Language, TokenType, HighlightToken};

// ============================================================================
// Language Registry - Bundled Grammars
// ============================================================================

/// Registry of all bundled Tree-sitter grammars
pub struct GrammarRegistry {
    languages: HashMap<&'static str, TsLanguage>,
    highlights: HashMap<&'static str, &'static str>,
}

impl GrammarRegistry {
    fn new() -> Self {
        let mut languages = HashMap::new();
        let mut highlights = HashMap::new();

        // Rust
        #[cfg(feature = "tree-sitter-rust")]
        {
            languages.insert("rust", tree_sitter_rust::LANGUAGE.into());
            highlights.insert("rust", include_str!("../../queries/rust/highlights.scm"));
        }

        // Python
        #[cfg(feature = "tree-sitter-python")]
        {
            languages.insert("python", tree_sitter_python::LANGUAGE.into());
            highlights.insert("python", include_str!("../../queries/python/highlights.scm"));
        }

        // JavaScript
        #[cfg(feature = "tree-sitter-javascript")]
        {
            languages.insert("javascript", tree_sitter_javascript::LANGUAGE.into());
            highlights.insert("javascript", include_str!("../../queries/javascript/highlights.scm"));
        }

        // TypeScript
        #[cfg(feature = "tree-sitter-typescript")]
        {
            languages.insert("typescript", tree_sitter_typescript::LANGUAGE.into());
            highlights.insert("typescript", include_str!("../../queries/typescript/highlights.scm"));
            
            languages.insert("tsx", tree_sitter_typescript::LANGUAGE_TSX.into());
            highlights.insert("tsx", include_str!("../../queries/tsx/highlights.scm"));
        }

        // Go
        #[cfg(feature = "tree-sitter-go")]
        {
            languages.insert("go", tree_sitter_go::LANGUAGE.into());
            highlights.insert("go", include_str!("../../queries/go/highlights.scm"));
        }

        // C
        #[cfg(feature = "tree-sitter-c")]
        {
            languages.insert("c", tree_sitter_c::LANGUAGE.into());
            highlights.insert("c", include_str!("../../queries/c/highlights.scm"));
        }

        // C++
        #[cfg(feature = "tree-sitter-cpp")]
        {
            languages.insert("cpp", tree_sitter_cpp::LANGUAGE.into());
            highlights.insert("cpp", include_str!("../../queries/cpp/highlights.scm"));
        }

        // C#
        #[cfg(feature = "tree-sitter-c-sharp")]
        {
            languages.insert("csharp", tree_sitter_c_sharp::LANGUAGE.into());
            highlights.insert("csharp", include_str!("../../queries/csharp/highlights.scm"));
        }

        // Java
        #[cfg(feature = "tree-sitter-java")]
        {
            languages.insert("java", tree_sitter_java::LANGUAGE.into());
            highlights.insert("java", include_str!("../../queries/java/highlights.scm"));
        }

        // Ruby
        #[cfg(feature = "tree-sitter-ruby")]
        {
            languages.insert("ruby", tree_sitter_ruby::LANGUAGE.into());
            highlights.insert("ruby", include_str!("../../queries/ruby/highlights.scm"));
        }

        // PHP
        #[cfg(feature = "tree-sitter-php")]
        {
            languages.insert("php", tree_sitter_php::LANGUAGE.into());
            highlights.insert("php", include_str!("../../queries/php/highlights.scm"));
        }

        // Swift
        #[cfg(feature = "tree-sitter-swift")]
        {
            languages.insert("swift", tree_sitter_swift::LANGUAGE.into());
            highlights.insert("swift", include_str!("../../queries/swift/highlights.scm"));
        }

        // Kotlin
        #[cfg(feature = "tree-sitter-kotlin")]
        {
            languages.insert("kotlin", tree_sitter_kotlin::LANGUAGE.into());
            highlights.insert("kotlin", include_str!("../../queries/kotlin/highlights.scm"));
        }

        // Dart
        #[cfg(feature = "tree-sitter-dart")]
        {
            languages.insert("dart", tree_sitter_dart::LANGUAGE.into());
            highlights.insert("dart", include_str!("../../queries/dart/highlights.scm"));
        }

        // HTML
        #[cfg(feature = "tree-sitter-html")]
        {
            languages.insert("html", tree_sitter_html::LANGUAGE.into());
            highlights.insert("html", include_str!("../../queries/html/highlights.scm"));
        }

        // CSS
        #[cfg(feature = "tree-sitter-css")]
        {
            languages.insert("css", tree_sitter_css::LANGUAGE.into());
            highlights.insert("css", include_str!("../../queries/css/highlights.scm"));
        }

        // JSON
        #[cfg(feature = "tree-sitter-json")]
        {
            languages.insert("json", tree_sitter_json::LANGUAGE.into());
            highlights.insert("json", include_str!("../../queries/json/highlights.scm"));
        }

        // YAML
        #[cfg(feature = "tree-sitter-yaml")]
        {
            languages.insert("yaml", tree_sitter_yaml::LANGUAGE.into());
            highlights.insert("yaml", include_str!("../../queries/yaml/highlights.scm"));
        }

        // TOML
        #[cfg(feature = "tree-sitter-toml")]
        {
            languages.insert("toml", tree_sitter_toml::LANGUAGE.into());
            highlights.insert("toml", include_str!("../../queries/toml/highlights.scm"));
        }

        // Markdown
        #[cfg(feature = "tree-sitter-markdown")]
        {
            languages.insert("markdown", tree_sitter_markdown::LANGUAGE.into());
            highlights.insert("markdown", include_str!("../../queries/markdown/highlights.scm"));
        }

        Self { languages, highlights }
    }

    pub fn global() -> &'static GrammarRegistry {
        static REGISTRY: OnceLock<GrammarRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            debug!("Initializing Tree-sitter grammar registry");
            GrammarRegistry::new()
        })
    }

    pub fn get_language(&self, name: &str) -> Option<&TsLanguage> {
        self.languages.get(name)
    }

    pub fn get_highlights(&self, name: &str) -> Option<&'static str> {
        self.highlights.get(name).copied()
    }

    pub fn has_language(&self, name: &str) -> bool {
        self.languages.contains_key(name)
    }
}

// ============================================================================
// Tree-sitter Highlighter
// ============================================================================

/// Tree-sitter highlighter for a specific language
#[derive(Clone)]
pub struct TreeSitterHighlighter {
    /// Language name
    language_name: String,
    /// Tree-sitter language
    ts_language: TsLanguage,
    /// Highlight query
    highlights_query: Arc<Query>,
    /// Parser
    parser: Arc<Mutex<Parser>>,
    /// Query manager
    query_manager: Arc<QueryManager>,
}

impl std::fmt::Debug for TreeSitterHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeSitterHighlighter")
            .field("language_name", &self.language_name)
            .field("ts_language", &"tree_sitter::Language")
            .field("highlights_query", &"Query")
            .field("parser", &"Parser")
            .field("query_manager", &"QueryManager")
            .finish()
    }
}

impl TreeSitterHighlighter {
    /// Create new Tree-sitter highlighter
    pub fn new(language: &Language) -> Option<Self> {
        let registry = GrammarRegistry::global();
        
        let language_name = language.name.as_str();
        if !registry.has_language(language_name) {
            warn!("No Tree-sitter grammar for language: {}", language_name);
            return None;
        }

        let ts_language = *registry.get_language(language_name)?;
        let highlights_src = registry.get_highlights(language_name)?;
        
        let highlights_query = Query::new(ts_language, highlights_src).ok()?;
        
        let mut parser = Parser::new();
        parser.set_language(ts_language).ok()?;
        
        Some(Self {
            language_name: language_name.to_string(),
            ts_language,
            highlights_query: Arc::new(highlights_query),
            parser: Arc::new(Mutex::new(parser)),
            query_manager: Arc::new(QueryManager::new()),
        })
    }

    /// Highlight a line using Tree-sitter
    pub fn highlight_line(&self, line: usize, text: &str) -> Vec<HighlightToken> {
        if text.is_empty() {
            return vec![];
        }

        // Parse the whole document (incremental parsing would be better)
        let mut parser = match self.parser.lock() {
            Ok(p) => p,
            Err(_) => return vec![self.fallback_token(text)],
        };

        let tree = match parser.parse(text, None) {
            Some(t) => t,
            None => return vec![self.fallback_token(text)],
        };

        // Get the root node
        let root = tree.root_node();
        
        // Find the node at the given line
        let line_start_byte = self.line_start_byte(text, line);
        let line_end_byte = self.line_end_byte(text, line);
        
        let mut tokens = Vec::new();
        let mut cursor = root.walk();
        
        // Traverse the tree and collect nodes that intersect this line
        self.collect_tokens_for_line(&mut cursor, line_start_byte, line_end_byte, text, &mut tokens);
        
        if tokens.is_empty() {
            vec![self.fallback_token(text)]
        } else {
            tokens
        }
    }

    /// Collect tokens for a specific line
    fn collect_tokens_for_line(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        line_start: usize,
        line_end: usize,
        text: &str,
        tokens: &mut Vec<HighlightToken>,
    ) {
        let node = cursor.node();
        let node_start = node.start_byte();
        let node_end = node.end_byte();
        
        // Check if node intersects this line
        if node_end > line_start && node_start < line_end {
            // Get the node's text
            let node_text = &text[node_start..node_end.min(text.len())];
            
            // Determine token type from node kind
            let token_type = self.node_kind_to_token_type(node.kind());
            
            // Create token
            tokens.push(HighlightToken {
                text: node_text.to_string(),
                token_type,
                start: node_start,
                end: node_end,
                style: Default::default(),
            });
        }
        
        // Recurse into children
        if cursor.goto_first_child() {
            loop {
                self.collect_tokens_for_line(cursor, line_start, line_end, text, tokens);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Convert node kind to token type
    fn node_kind_to_token_type(&self, kind: &str) -> TokenType {
        match kind {
            // Keywords
            "fn" | "let" | "mut" | "if" | "else" | "for" | "while" | "loop"
            | "match" | "pub" | "use" | "mod" | "struct" | "enum" | "impl"
            | "return" | "self" | "async" | "await" | "dyn" | "trait"
            | "def" | "class" | "import" | "from" | "as" | "try" | "except"
            | "with" | "lambda" | "yield" | "global" | "nonlocal"
            | "var" | "const" | "switch" | "case" | "default" | "break" | "continue" => TokenType::Keyword,
            
            // Functions
            "function" | "function_declaration" | "method" | "method_declaration"
            | "call" | "call_expression" => TokenType::Function,
            
            // Types
            "type" | "type_identifier" | "type_parameter" | "type_argument"
            | "union" | "interface" => TokenType::Type,
            
            // Variables
            "identifier" | "variable" | "variable_name" => TokenType::Variable,
            
            // Constants
            "constant" | "static" => TokenType::Constant,
            
            // Strings
            "string" | "string_literal" | "raw_string" | "template_string"
            | "char" | "character" => TokenType::String,
            
            // Comments
            "comment" | "line_comment" | "block_comment" => TokenType::Comment,
            
            // Numbers
            "number" | "integer" | "float" | "numeric_literal" => TokenType::Number,
            
            // Operators
            "operator" | "=" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
            | "!" | "~" | "<" | ">" | "==" | "!=" | "<=" | ">=" | "&&" | "||" => TokenType::Operator,
            
            // Punctuation
            ";" | "," | "." | ":" | "::" | "->" | "=>" => TokenType::Punctuation,
            
            // Brackets
            "(" | ")" | "[" | "]" | "{" | "}" => TokenType::Bracket,
            
            // Annotations
            "attribute" | "annotation" | "decorator" => TokenType::Annotation,
            
            // Preprocessor
            "preproc" | "preproc_arg" | "include" => TokenType::Preprocessor,
            
            // Whitespace
            "whitespace" | "\n" | "\r" | "\t" | " " => TokenType::Whitespace,
            
            _ => TokenType::None,
        }
    }

    /// Fallback token when Tree-sitter fails
    fn fallback_token(&self, text: &str) -> HighlightToken {
        HighlightToken {
            text: text.to_string(),
            token_type: TokenType::None,
            start: 0,
            end: text.len(),
            style: Default::default(),
        }
    }

    /// Get byte offset for start of line
    fn line_start_byte(&self, text: &str, line: usize) -> usize {
        let mut byte = 0;
        let mut current_line = 0;
        
        for c in text.chars() {
            if current_line == line {
                break;
            }
            byte += c.len_utf8();
            if c == '\n' {
                current_line += 1;
            }
        }
        
        byte
    }

    /// Get byte offset for end of line
    fn line_end_byte(&self, text: &str, line: usize) -> usize {
        let mut current_line = 0;
        let mut last_byte = text.len();
        
        for (i, c) in text.char_indices() {
            if current_line == line && c == '\n' {
                last_byte = i;
                break;
            }
            if c == '\n' {
                current_line += 1;
            }
        }
        
        last_byte
    }

    /// Parse full document
    pub fn parse(&mut self, source: &str) -> Result<Tree> {
        let mut parser = self.parser.lock().map_err(|_| anyhow!("Failed to lock parser"))?;
        parser.parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse document"))
    }

    /// Get node at position
    pub fn node_at_position<'a>(&self, tree: &'a Tree, line: usize, column: usize) -> Option<Node<'a>> {
        let byte = self.position_to_byte(tree, line, column)?;
        tree.root_node().descendant_for_byte_range(byte, byte)
    }

    /// Convert line/column to byte offset
    fn position_to_byte(&self, tree: &Tree, line: usize, column: usize) -> Option<usize> {
        let root = tree.root_node();
        let mut line_count = 0;
        let mut byte = 0;

        // Walk through children to find the line
        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                if line_count == line {
                    return Some(byte + column);
                }
                line_count += 1;
                byte += child.end_byte();
            }
        }

        None
    }

    /// Get token type from capture name
    fn capture_to_token_type(capture: &str) -> TokenType {
        match capture {
            "keyword" => TokenType::Keyword,
            "function" => TokenType::Function,
            "type" => TokenType::Type,
            "variable" => TokenType::Variable,
            "constant" => TokenType::Constant,
            "string" => TokenType::String,
            "comment" => TokenType::Comment,
            "number" => TokenType::Number,
            "operator" => TokenType::Operator,
            "punctuation" => TokenType::Punctuation,
            "bracket" => TokenType::Bracket,
            "annotation" => TokenType::Annotation,
            "preprocessor" => TokenType::Preprocessor,
            "whitespace" => TokenType::Whitespace,
            "error" => TokenType::Error,
            _ => TokenType::None,
        }
    }
}

// ============================================================================
// Query Manager
// ============================================================================

/// Tree-sitter query manager
#[derive(Default)]
pub struct QueryManager {
    /// Loaded queries
    queries: HashMap<String, Query>,
    /// Query sources
    sources: HashMap<String, String>,
}

impl QueryManager {
    pub fn new() -> Self {
        Self {
            queries: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// Load query from string
    pub fn load_query(&mut self, name: &str, source: &str, lang: &TsLanguage) -> Result<()> {
        let query = Query::new(*lang, source)?;
        self.queries.insert(name.to_string(), query);
        self.sources.insert(name.to_string(), source.to_string());
        Ok(())
    }

    /// Get query by name
    pub fn get_query(&self, name: &str) -> Option<&Query> {
        self.queries.get(name)
    }

    /// Get query source
    pub fn get_source(&self, name: &str) -> Option<&str> {
        self.sources.get(name).map(|s| s.as_str())
    }

    /// Check if query exists
    pub fn has_query(&self, name: &str) -> bool {
        self.queries.contains_key(name)
    }
}

// ============================================================================
// Tree Cache
// ============================================================================

/// Tree-sitter syntax tree cache
pub struct TreeCache {
    /// Cached trees by document version
    trees: HashMap<usize, (Tree, usize)>,
    /// Maximum cache size
    max_size: usize,
}

impl TreeCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            trees: HashMap::with_capacity(max_size),
            max_size,
        }
    }

    /// Get cached tree for version
    pub fn get(&self, version: usize) -> Option<&Tree> {
        self.trees.get(&version).map(|(tree, _)| tree)
    }

    /// Insert tree for version
    pub fn insert(&mut self, version: usize, tree: Tree) {
        if self.trees.len() >= self.max_size {
            // Remove oldest entry
            if let Some(oldest) = self.trees.keys().next().copied() {
                self.trees.remove(&oldest);
            }
        }
        self.trees.insert(version, (tree, version));
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.trees.clear();
    }

    /// Remove versions older than given
    pub fn retain_recent(&mut self, keep: usize) {
        self.trees.retain(|_, (_, ver)| *ver >= keep);
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.trees.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }
}

impl Default for TreeCache {
    fn default() -> Self {
        Self::new(10)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_registry() {
        let registry = GrammarRegistry::global();
        // At minimum, Rust should be available
        #[cfg(feature = "tree-sitter-rust")]
        {
            assert!(registry.has_language("rust"));
            assert!(registry.get_highlights("rust").is_some());
        }
    }

    #[test]
fn test_highlighter_creation() {
        let lang = Language::rust();
        let highlighter = TreeSitterHighlighter::new(&lang);
        
        #[cfg(feature = "tree-sitter-rust")]
        {
            assert!(highlighter.is_some());
        }
        
        #[cfg(not(feature = "tree-sitter-rust"))]
        {
            assert!(highlighter.is_none());
        }
    }

    #[test]
    fn test_highlight_line() {
        let lang = Language::rust();
        if let Some(highlighter) = TreeSitterHighlighter::new(&lang) {
            let tokens = highlighter.highlight_line(0, "fn main() {}");
            assert!(!tokens.is_empty());
        }
    }

    #[test]
    fn test_tree_cache() {
        let mut cache = TreeCache::new(3);
        assert_eq!(cache.len(), 0);
        
        // Create dummy tree (would need actual tree in real test)
        // For now, just test cache operations
        #[allow(invalid_value)]
        unsafe {
            cache.insert(1, std::mem::zeroed());
        }
        assert_eq!(cache.len(), 1);
        
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_query_manager() {
        let mut qm = QueryManager::new();
        assert_eq!(qm.queries.len(), 0);
        
        // Would need actual language and query for real test
        // For now, just test the interface
    }
}