# Parsec IDE - Complete Error Fix Script (Safe Version)
# Run from root directory: C:\Users\divyaamshu\Desktop\VS Code\Code

Write-Host "🔧 Parsec IDE - Complete Error Fix Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if (-not (Test-Path "Cargo.toml")) {
    Write-Host "❌ Error: Run this script from the root directory (where Cargo.toml exists)" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Found root Cargo.toml" -ForegroundColor Green

# Helper to write Rust file safely
function Write-RustFile {
    param($Path, $Content)
    # Replace backticks to avoid PowerShell escaping issues
    $Content = $Content -replace '`', '``'
    [System.IO.File]::WriteAllText($Path, $Content)
}

# ============================================================================
# 1. Create missing Git modules
# ============================================================================
Write-Host "`n📁 Creating missing Git module files..." -ForegroundColor Yellow

$gitDir = "core\src\git"
if (-not (Test-Path $gitDir)) { New-Item -ItemType Directory -Path $gitDir -Force | Out-Null }

$commitContent = @'
//! Git commit types
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub summary: String,
    pub body: Option<String>,
    pub author_name: String,
    pub author_email: String,
    pub author_time: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_time: DateTime<Utc>,
    pub parents: usize,
    pub parent_ids: Vec<String>,
    pub tree_id: String,
}

impl Commit {
    pub fn format_short(&self) -> String {
        format!("{} {}", self.short_id, self.summary)
    }
}
'@
Write-RustFile -Path "$gitDir\commit.rs" -Content $commitContent
Write-Host "  ✅ Created commit.rs"

$remoteContent = @'
//! Git remote types
#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

impl Remote {
    pub fn is_github(&self) -> bool {
        self.url.contains("github.com")
    }
    pub fn is_gitlab(&self) -> bool {
        self.url.contains("gitlab.com")
    }
}
'@
Write-RustFile -Path "$gitDir\remote.rs" -Content $remoteContent
Write-Host "  ✅ Created remote.rs"

$diffContent = @'
//! Git diff types
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
    Header,
}
'@
Write-RustFile -Path "$gitDir\diff.rs" -Content $diffContent
Write-Host "  ✅ Created diff.rs"

$stashContent = @'
//! Git stash types
use super::Commit;

#[derive(Debug, Clone)]
pub struct Stash {
    pub id: String,
    pub index: usize,
    pub message: String,
    pub branch: String,
    pub commit: Commit,
}

impl Stash {
    pub fn display_name(&self) -> String {
        format!("stash@{{{}}}: {}", self.index, self.message)
    }
}
'@
Write-RustFile -Path "$gitDir\stash.rs" -Content $stashContent
Write-Host "  ✅ Created stash.rs"

# Update git/mod.rs to include new modules (if needed)
$gitModFile = "$gitDir\mod.rs"
if (Test-Path $gitModFile) {
    $content = Get-Content $gitModFile -Raw
    if ($content -notmatch "mod commit;") {
        # Use simple concatenation with newline
        $content = "mod commit;`r`nmod remote;`r`nmod diff;`r`nmod stash;`r`n" + $content
        $content | Out-File -FilePath $gitModFile -Encoding UTF8
        Write-Host "  ✅ Updated git/mod.rs"
    }
}

# ============================================================================
# 2. Create missing syntax HighlightStyle
# ============================================================================
Write-Host "`n📁 Creating missing syntax files..." -ForegroundColor Yellow

$syntaxDir = "core\src\syntax"
if (-not (Test-Path $syntaxDir)) { New-Item -ItemType Directory -Path $syntaxDir -Force | Out-Null }

$highlightContent = @'
//! Highlight style types
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightStyle {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Default for HighlightStyle {
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
'@
Write-RustFile -Path "$syntaxDir\highlight_style.rs" -Content $highlightContent
Write-Host "  ✅ Created highlight_style.rs"

# Update syntax/mod.rs to export HighlightStyle
$syntaxModFile = "$syntaxDir\mod.rs"
if (Test-Path $syntaxModFile) {
    $content = Get-Content $syntaxModFile -Raw
    if ($content -notmatch "pub use highlight_style::HighlightStyle;") {
        # Add mod declaration and pub use at top
        $content = "mod highlight_style;`r`npub use highlight_style::HighlightStyle;`r`n" + $content
        $content | Out-File -FilePath $syntaxModFile -Encoding UTF8
        Write-Host "  ✅ Updated syntax/mod.rs"
    }
}

# ============================================================================
# 3. Fix buffer.rs - Add missing imports
# ============================================================================
Write-Host "`n📁 Fixing editor/buffer.rs..." -ForegroundColor Yellow

$bufferFile = "core\src\editor\buffer.rs"
if (Test-Path $bufferFile) {
    $content = Get-Content $bufferFile -Raw
    if ($content -notmatch "use std::sync::atomic::\{AtomicUsize, Ordering\};") {
        # Insert after existing use statements
        $newContent = $content -replace "(use[^;]+;)(\r?\n)", "`$1`$2use std::sync::atomic::{AtomicUsize, Ordering};`$2"
        if ($newContent -notmatch "static NEXT_ID: AtomicUsize") {
            $newContent = "static NEXT_ID: AtomicUsize = AtomicUsize::new(1);`r`n`r`n" + $newContent
        }
        $newContent | Out-File -FilePath $bufferFile -Encoding UTF8
        Write-Host "  ✅ Fixed buffer.rs"
    } else {
        Write-Host "  ✅ buffer.rs already has imports"
    }
}

# ============================================================================
# 4. Fix lib.rs - Update function calls and mutable borrow
# ============================================================================
Write-Host "`n📁 Fixing lib.rs..." -ForegroundColor Yellow

$libFile = "core\src\lib.rs"
if (Test-Path $libFile) {
    $content = Get-Content $libFile -Raw
    Copy-Item $libFile "$libFile.bak" -Force

    # Simple replacements (use single quotes for literal strings)
    $content = $content.Replace('terminal::Terminal::new()', 'terminal::Terminal::new("term-1".to_string(), "Terminal".to_string(), terminal::TerminalConfig::default())')
    $content = $content.Replace('git::GitManager::new()', 'git::GitManager::new(git::GitConfig::default())')
    $content = $content.Replace('syntax::SyntaxSystem::new()', 'syntax::SyntaxSystem::new(syntax::SyntaxConfig::default())')

    $content = $content.Replace('pub editor: Arc<Editor>', 'pub editor: Arc<tokio::sync::Mutex<Editor>>')
    $content = $content.Replace('editor: Arc::new(Editor::new())', 'editor: Arc::new(tokio::sync::Mutex::new(Editor::new()))')

    $content = $content.Replace('self.editor.open_file', 'self.editor.lock().await.open_file')
    $content = $content.Replace('self.editor.get_content', 'self.editor.lock().await.get_content')
    $content = $content.Replace('self.editor.insert', 'self.editor.lock().await.insert')
    $content = $content.Replace('self.editor.save_current', 'self.editor.lock().await.save_current')

    # Fix open_file function (use regex but with escaped brackets)
    $content = $content -replace 'pub async fn open_file<P: AsRef<std::path::Path>>\(&self, path: P\) -> Result<\(\)> \{[^}]*\}', @'
pub async fn open_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
    self.editor.lock().await.open_file(path).await?;
    Ok(())
}
'@

    $content | Out-File -FilePath $libFile -Encoding UTF8
    Write-Host "  ✅ Fixed lib.rs"
}

# ============================================================================
# 5. Fix process.rs - Add imports and fix match statements
# ============================================================================
Write-Host "`n📁 Fixing process.rs..." -ForegroundColor Yellow

$processFile = "core\src\process.rs"
if (Test-Path $processFile) {
    $content = Get-Content $processFile -Raw
    if ($content -notmatch "use futures::stream::StreamExt;") {
        $content = "use futures::stream::StreamExt;`r`n" + $content
    }

    # Use regex with careful escaping
    $content = $content -replace 'pub async fn is_running\(&self\) -> bool \{[^}]*\}', @'
pub async fn is_running(&self) -> bool {
    match *self.status.lock().await {
        ProcessStatus::Running => true,
        _ => false,
    }
}
'@

    $content = $content -replace 'pub async fn exit_status\(&self\) -> Option<i32> \{[^}]*\}', @'
pub async fn exit_status(&self) -> Option<i32> {
    match *self.status.lock().await {
        ProcessStatus::Exited(code) => Some(code),
        _ => None,
    }
}
'@

    if ($content -notmatch "pub type CommandHandle =") {
        $content = $content -replace '(pub struct ProcessManager \{[^}]*\})', @'

/// Handle to a running command
pub type CommandHandle = usize;

$1
'@
    }

    $content | Out-File -FilePath $processFile -Encoding UTF8
    Write-Host "  ✅ Fixed process.rs"
}

# ============================================================================
# 6. Fix terminal/renderer.rs - Use string .Replace()
# ============================================================================
Write-Host "`n📁 Fixing terminal/renderer.rs..." -ForegroundColor Yellow

$rendererFile = "core\src\terminal\renderer.rs"
if (Test-Path $rendererFile) {
    $content = Get-Content $rendererFile -Raw

    $content = $content.Replace(
        'Color::Indexed(n) => codes.push(&format!("38;5;{}", n)),',
        'Color::Indexed(n) => { let code = format!("38;5;{}", n); codes.push(&code); },'
    )
    $content = $content.Replace(
        'Color::Rgb(r, g, b) => codes.push(&format!("38;2;{};{};{}", r, g, b)),',
        'Color::Rgb(r, g, b) => { let code = format!("38;2;{};{};{}", r, g, b); codes.push(&code); },'
    )
    $content = $content.Replace(
        'Color::Indexed(n) => codes.push(&format!("48;5;{}", n)),',
        'Color::Indexed(n) => { let code = format!("48;5;{}", n); codes.push(&code); },'
    )
    $content = $content.Replace(
        'Color::Rgb(r, g, b) => codes.push(&format!("48;2;{};{};{}", r, g, b)),',
        'Color::Rgb(r, g, b) => { let code = format!("48;2;{};{};{}", r, g, b); codes.push(&code); },'
    )

    $content | Out-File -FilePath $rendererFile -Encoding UTF8
    Write-Host "  ✅ Fixed renderer.rs"
}

# ============================================================================
# 7. Add Default for EOL
# ============================================================================
Write-Host "`n📁 Adding Default implementation for EOL..." -ForegroundColor Yellow

$editorModFile = "core\src\editor\mod.rs"
if (Test-Path $editorModFile) {
    $content = Get-Content $editorModFile -Raw
    if ($content -notmatch "impl Default for EOL") {
        $eolImpl = @'

impl Default for EOL {
    fn default() -> Self {
        if cfg!(windows) {
            EOL::CRLF
        } else {
            EOL::LF
        }
    }
}
'@
        $content += $eolImpl
        $content | Out-File -FilePath $editorModFile -Encoding UTF8
        Write-Host "  ✅ Added Default for EOL"
    }
}

# ============================================================================
# 8. Remove Default from EditorStats
# ============================================================================
Write-Host "`n📁 Fixing EditorMode Default issue..." -ForegroundColor Yellow

if (Test-Path $editorModFile) {
    $content = Get-Content $editorModFile -Raw
    $content = $content -replace '#\[derive\(Debug, Clone, Default\)\]', '#[derive(Debug, Clone)]'
    $content | Out-File -FilePath $editorModFile -Encoding UTF8
    Write-Host "  ✅ Removed Default from EditorStats"
}

# ============================================================================
# 9. Add Debug derives to terminal types
# ============================================================================
Write-Host "`n📁 Adding Debug derives to terminal types..." -ForegroundColor Yellow

$terminalBufferFile = "core\src\terminal\buffer.rs"
if (Test-Path $terminalBufferFile -and (Get-Content $terminalBufferFile -Raw) -notmatch "#\[derive\(Debug\)\]") {
    $content = Get-Content $terminalBufferFile -Raw
    $content = $content -replace "pub struct TerminalBuffer \{$", "#[derive(Debug)]`r`npub struct TerminalBuffer {"
    $content | Out-File -FilePath $terminalBufferFile -Encoding UTF8
    Write-Host "  ✅ Added Debug to TerminalBuffer"
}

$terminalRendererFile = "core\src\terminal\renderer.rs"
if (Test-Path $terminalRendererFile -and (Get-Content $terminalRendererFile -Raw) -notmatch "#\[derive\(Debug\)\]") {
    $content = Get-Content $terminalRendererFile -Raw
    $content = $content -replace "pub struct TerminalRenderer \{$", "#[derive(Debug)]`r`npub struct TerminalRenderer {"
    $content | Out-File -FilePath $terminalRendererFile -Encoding UTF8
    Write-Host "  ✅ Added Debug to TerminalRenderer"
}

# ============================================================================
# 10. Fix syntax/treesitter.rs
# ============================================================================
Write-Host "`n📁 Fixing syntax/treesitter.rs..." -ForegroundColor Yellow

$treesitterFile = "core\src\syntax\treesitter.rs"
if (Test-Path $treesitterFile) {
    $content = Get-Content $treesitterFile -Raw
    $content = $content -replace '#\[derive\(Debug, Clone\)\]', '#[derive(Debug)]'
    $content = $content -replace 'pub fn node_at_position\(&self, tree: &Tree, line: usize, column: usize\) -> Option<Node> \{', 
        "pub fn node_at_position<'a>(&self, tree: &'a Tree, line: usize, column: usize) -> Option<Node<'a>> {"
    $content = $content -replace 'Query::new\(lang, source\)', 'Query::new(*lang, source)'
    $content | Out-File -FilePath $treesitterFile -Encoding UTF8
    Write-Host "  ✅ Fixed treesitter.rs"
}

# ============================================================================
# 11. Add allow(unused) to quiet warnings
# ============================================================================
Write-Host "`n📁 Adding allow(unused) attributes..." -ForegroundColor Yellow

$files = @(
    "core\src\lib.rs",
    "core\src\editor\buffer.rs",
    "core\src\editor\selection.rs",
    "core\src\editor\history.rs",
    "core\src\editor\position.rs",
    "core\src\terminal\pty.rs",
    "core\src\terminal\mod.rs",
    "core\src\git\repository.rs",
    "core\src\git\mod.rs",
    "core\src\syntax\treesitter.rs",
    "core\src\syntax\mod.rs"
)

foreach ($file in $files) {
    if (Test-Path $file) {
        $content = Get-Content $file -Raw
        if ($content -notmatch '#!\[allow\(unused\)\]') {
            $content = "#![allow(unused)]`r`n" + $content
            $content | Out-File -FilePath $file -Encoding UTF8
            Write-Host "  ✅ Added allow(unused) to $file"
        }
    }
}

# ============================================================================
# 12. Final cleanup
# ============================================================================
Write-Host "`n🧹 Running cargo clean..." -ForegroundColor Yellow
cargo clean

Write-Host "`n✅ All fixes applied! Now run: cargo build" -ForegroundColor Green