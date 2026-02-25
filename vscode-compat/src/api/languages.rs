//! VS Code Languages API Implementation
//!
//! Implements vscode.languages.* API for language features like completion, hover, etc.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use async_trait::async_trait;

use crate::api::{TextDocument, VSCodePosition, Disposable, CompletionItem, Hover, MarkdownString};

/// Languages API implementation
pub struct LanguagesAPI {
    /// Completion providers
    completion_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn CompletionProvider>>>>>,
    /// Hover providers
    hover_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn HoverProvider>>>>>,
    /// Signature help providers
    signature_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn SignatureHelpProvider>>>>>,
    /// Definition providers
    definition_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn DefinitionProvider>>>>>,
    /// Reference providers
    reference_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn ReferenceProvider>>>>>,
    /// Document symbol providers
    symbol_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn DocumentSymbolProvider>>>>>,
    /// Code action providers
    code_action_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn CodeActionProvider>>>>>,
    /// Formatting providers
    formatting_providers: Arc<RwLock<HashMap<String, Vec<Box<dyn DocumentFormattingProvider>>>>>,
}

/// Language selector
#[derive(Debug, Clone)]
pub struct DocumentSelector {
    pub language: Option<String>,
    pub scheme: Option<String>,
    pub pattern: Option<String>,
}

/// Completion provider trait
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    async fn provide_completion_items(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Vec<CompletionItem>>;
}

/// Hover provider trait
#[async_trait]
pub trait HoverProvider: Send + Sync {
    async fn provide_hover(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Option<Hover>>;
}

/// Signature help provider trait
#[async_trait]
pub trait SignatureHelpProvider: Send + Sync {
    async fn provide_signature_help(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Option<SignatureHelp>>;
}

/// Definition provider trait
#[async_trait]
pub trait DefinitionProvider: Send + Sync {
    async fn provide_definition(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Vec<Location>>;
}

/// Reference provider trait
#[async_trait]
pub trait ReferenceProvider: Send + Sync {
    async fn provide_references(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
        include_declaration: bool,
    ) -> Result<Vec<Location>>;
}

/// Document symbol provider trait
#[async_trait]
pub trait DocumentSymbolProvider: Send + Sync {
    async fn provide_document_symbols(
        &self,
        document: &TextDocument,
    ) -> Result<Vec<DocumentSymbol>>;
}

/// Code action provider trait
#[async_trait]
pub trait CodeActionProvider: Send + Sync {
    async fn provide_code_actions(
        &self,
        document: &TextDocument,
        range: crate::api::VSCodeRange,
        context: CodeActionContext,
    ) -> Result<Vec<CodeAction>>;
}

/// Document formatting provider trait
#[async_trait]
pub trait DocumentFormattingProvider: Send + Sync {
    async fn provide_document_formatting(
        &self,
        document: &TextDocument,
        options: FormattingOptions,
    ) -> Result<Vec<TextEdit>>;
}

/// Signature help
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
    pub active_signature: usize,
    pub active_parameter: usize,
}

/// Signature information
#[derive(Debug, Clone)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInformation>,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct ParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

/// Location
#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: crate::api::VSCodeRange,
}

/// Document symbol
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: SymbolKind,
    pub range: crate::api::VSCodeRange,
    pub selection_range: crate::api::VSCodeRange,
    pub children: Vec<DocumentSymbol>,
}

/// Symbol kind
#[derive(Debug, Clone, Copy)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// Code action context
#[derive(Debug, Clone)]
pub struct CodeActionContext {
    pub diagnostics: Vec<Diagnostic>,
    pub only: Option<Vec<String>>,
}

/// Diagnostic
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: crate::api::VSCodeRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Code action
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub diagnostics: Option<Vec<Diagnostic>>,
    pub edit: Option<super::WorkspaceEdit>,
    pub command: Option<super::Command>,
    pub is_preferred: bool,
}

/// Formatting options
#[derive(Debug, Clone)]
pub struct FormattingOptions {
    pub tab_size: usize,
    pub insert_spaces: bool,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
    pub trim_final_newlines: bool,
}

/// Text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: crate::api::VSCodeRange,
    pub new_text: String,
}

impl LanguagesAPI {
    pub fn new() -> Self {
        Self {
            completion_providers: Arc::new(RwLock::new(HashMap::new())),
            hover_providers: Arc::new(RwLock::new(HashMap::new())),
            signature_providers: Arc::new(RwLock::new(HashMap::new())),
            definition_providers: Arc::new(RwLock::new(HashMap::new())),
            reference_providers: Arc::new(RwLock::new(HashMap::new())),
            symbol_providers: Arc::new(RwLock::new(HashMap::new())),
            code_action_providers: Arc::new(RwLock::new(HashMap::new())),
            formatting_providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ==================== Completion ====================

    /// Register a completion provider
    pub async fn register_completion_provider(
        &self,
        selector: Vec<String>,
        provider: impl CompletionProvider + Send + Sync + 'static,
    ) -> impl Disposable {
        for lang in selector {
            self.completion_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct CompletionDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn CompletionProvider>>>>>,
        }

        impl Disposable for CompletionDisposable {
            fn dispose(&self) {
                // Remove providers
                // This is simplified - real implementation would need to track specific providers
            }
        }

        CompletionDisposable {
            selector,
            providers: self.completion_providers.clone(),
        }
    }

    /// Provide completion items
    pub async fn provide_completion_items(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Vec<CompletionItem>> {
        let mut all_items = Vec::new();

        if let Some(providers) = self.completion_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(items) = provider.provide_completion_items(document, position).await {
                    all_items.extend(items);
                }
            }
        }

        Ok(all_items)
    }

    // ==================== Hover ====================

    /// Register a hover provider
    pub async fn register_hover_provider(
        &self,
        selector: Vec<String>,
        provider: impl HoverProvider + Send + Sync + 'static,
    ) -> impl Disposable {
        for lang in selector {
            self.hover_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct HoverDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn HoverProvider>>>>>,
        }

        impl Disposable for HoverDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        HoverDisposable {
            selector,
            providers: self.hover_providers.clone(),
        }
    }

    /// Provide hover
    pub async fn provide_hover(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Option<Hover>> {
        if let Some(providers) = self.hover_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(Some(hover)) = provider.provide_hover(document, position).await {
                    return Ok(Some(hover));
                }
            }
        }

        Ok(None)
    }

    // ==================== Signature Help ====================

    /// Register a signature help provider
    pub async fn register_signature_help_provider(
        &self,
        selector: Vec<String>,
        provider: impl SignatureHelpProvider + Send + Sync + 'static,
        trigger_characters: Option<Vec<String>>,
    ) -> impl Disposable {
        for lang in selector {
            self.signature_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct SignatureDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn SignatureHelpProvider>>>>>,
        }

        impl Disposable for SignatureDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        SignatureDisposable {
            selector,
            providers: self.signature_providers.clone(),
        }
    }

    /// Provide signature help
    pub async fn provide_signature_help(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Option<SignatureHelp>> {
        if let Some(providers) = self.signature_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(Some(sig)) = provider.provide_signature_help(document, position).await {
                    return Ok(Some(sig));
                }
            }
        }

        Ok(None)
    }

    // ==================== Definition ====================

    /// Register a definition provider
    pub async fn register_definition_provider(
        &self,
        selector: Vec<String>,
        provider: impl DefinitionProvider + Send + Sync + 'static,
    ) -> impl Disposable {
        for lang in selector {
            self.definition_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct DefinitionDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn DefinitionProvider>>>>>,
        }

        impl Disposable for DefinitionDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        DefinitionDisposable {
            selector,
            providers: self.definition_providers.clone(),
        }
    }

    /// Provide definition
    pub async fn provide_definition(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
    ) -> Result<Vec<Location>> {
        let mut all_locations = Vec::new();

        if let Some(providers) = self.definition_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(locs) = provider.provide_definition(document, position).await {
                    all_locations.extend(locs);
                }
            }
        }

        Ok(all_locations)
    }

    // ==================== References ====================

    /// Register a reference provider
    pub async fn register_reference_provider(
        &self,
        selector: Vec<String>,
        provider: impl ReferenceProvider + Send + Sync + 'static,
    ) -> impl Disposable {
        for lang in selector {
            self.reference_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct ReferenceDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn ReferenceProvider>>>>>,
        }

        impl Disposable for ReferenceDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        ReferenceDisposable {
            selector,
            providers: self.reference_providers.clone(),
        }
    }

    /// Provide references
    pub async fn provide_references(
        &self,
        document: &TextDocument,
        position: VSCodePosition,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let mut all_locations = Vec::new();

        if let Some(providers) = self.reference_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(locs) = provider.provide_references(document, position, include_declaration).await {
                    all_locations.extend(locs);
                }
            }
        }

        Ok(all_locations)
    }

    // ==================== Document Symbols ====================

    /// Register a document symbol provider
    pub async fn register_document_symbol_provider(
        &self,
        selector: Vec<String>,
        provider: impl DocumentSymbolProvider + Send + Sync + 'static,
    ) -> impl Disposable {
        for lang in selector {
            self.symbol_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct SymbolDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn DocumentSymbolProvider>>>>>,
        }

        impl Disposable for SymbolDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        SymbolDisposable {
            selector,
            providers: self.symbol_providers.clone(),
        }
    }

    /// Provide document symbols
    pub async fn provide_document_symbols(
        &self,
        document: &TextDocument,
    ) -> Result<Vec<DocumentSymbol>> {
        let mut all_symbols = Vec::new();

        if let Some(providers) = self.symbol_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(syms) = provider.provide_document_symbols(document).await {
                    all_symbols.extend(syms);
                }
            }
        }

        Ok(all_symbols)
    }

    // ==================== Code Actions ====================

    /// Register a code action provider
    pub async fn register_code_action_provider(
        &self,
        selector: Vec<String>,
        provider: impl CodeActionProvider + Send + Sync + 'static,
        metadata: Option<CodeActionProviderMetadata>,
    ) -> impl Disposable {
        for lang in selector {
            self.code_action_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct CodeActionDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn CodeActionProvider>>>>>,
        }

        impl Disposable for CodeActionDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        CodeActionDisposable {
            selector,
            providers: self.code_action_providers.clone(),
        }
    }

    /// Provide code actions
    pub async fn provide_code_actions(
        &self,
        document: &TextDocument,
        range: crate::api::VSCodeRange,
        context: CodeActionContext,
    ) -> Result<Vec<CodeAction>> {
        let mut all_actions = Vec::new();

        if let Some(providers) = self.code_action_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(actions) = provider.provide_code_actions(document, range, context.clone()).await {
                    all_actions.extend(actions);
                }
            }
        }

        Ok(all_actions)
    }

    // ==================== Formatting ====================

    /// Register a document formatting provider
    pub async fn register_document_formatting_provider(
        &self,
        selector: Vec<String>,
        provider: impl DocumentFormattingProvider + Send + Sync + 'static,
    ) -> impl Disposable {
        for lang in selector {
            self.formatting_providers.write().await
                .entry(lang)
                .or_insert_with(Vec::new)
                .push(Box::new(provider.clone_box()));
        }

        struct FormattingDisposable {
            selector: Vec<String>,
            providers: Arc<RwLock<HashMap<String, Vec<Box<dyn DocumentFormattingProvider>>>>>,
        }

        impl Disposable for FormattingDisposable {
            fn dispose(&self) {
                // Remove providers
            }
        }

        FormattingDisposable {
            selector,
            providers: self.formatting_providers.clone(),
        }
    }

    /// Provide document formatting
    pub async fn provide_document_formatting(
        &self,
        document: &TextDocument,
        options: FormattingOptions,
    ) -> Result<Vec<TextEdit>> {
        let mut all_edits = Vec::new();

        if let Some(providers) = self.formatting_providers.read().await.get(&document.language_id) {
            for provider in providers {
                if let Ok(edits) = provider.provide_document_formatting(document, options.clone()).await {
                    all_edits.extend(edits);
                }
            }
        }

        Ok(all_edits)
    }

    // ==================== Utility ====================

    /// Match a document against a selector
    pub fn match_selector(selector: &DocumentSelector, document: &TextDocument) -> bool {
        if let Some(lang) = &selector.language {
            if lang != &document.language_id {
                return false;
            }
        }

        if let Some(scheme) = &selector.scheme {
            if scheme != &document.uri.scheme {
                return false;
            }
        }

        if let Some(pattern) = &selector.pattern {
            if !document.file_name.contains(pattern) {
                return false;
            }
        }

        true
    }
}

impl Default for LanguagesAPI {
    fn default() -> Self {
        Self::new()
    }
}

/// Code action provider metadata
#[derive(Debug, Clone)]
pub struct CodeActionProviderMetadata {
    pub provided_code_action_kinds: Option<Vec<String>>,
}

/// Helper trait for cloning boxed providers
trait CloneBox {
    fn clone_box(&self) -> Box<dyn CompletionProvider>;
}

impl<T> CloneBox for T
where
    T: CompletionProvider + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn CompletionProvider> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCompletionProvider;

    #[async_trait]
    impl CompletionProvider for TestCompletionProvider {
        async fn provide_completion_items(
            &self,
            _document: &TextDocument,
            _position: VSCodePosition,
        ) -> Result<Vec<CompletionItem>> {
            Ok(vec![CompletionItem {
                label: "test".to_string(),
                kind: CompletionItemKind::Text,
                detail: None,
                documentation: None,
                sort_text: None,
                filter_text: None,
                insert_text: None,
                range: None,
                commit_characters: None,
                command: None,
            }])
        }
    }

    #[tokio::test]
    async fn test_completion_provider() {
        let api = LanguagesAPI::new();
        let selector = vec!["rust".to_string()];

        let _disposable = api.register_completion_provider(selector, TestCompletionProvider).await;

        // This test would need a real document
        // For now, just verify no panic
    }
}