//! CLI argument definitions

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::str::FromStr;

/// Parsec CLI - Lightning-fast IDE
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text, global = true)]
    pub format: OutputFormat,
    
    /// No color output
    #[arg(long, global = true)]
    pub no_color: bool,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    /// Quiet mode
    #[arg(short, long, global = true)]
    pub quiet: bool,
    
    #[command(subcommand)]
    pub command: Commands,
}

/// Output format
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
    Table,
    Quiet,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "yaml" | "yml" => Ok(OutputFormat::Yaml),
            "table" | "tbl" => Ok(OutputFormat::Table),
            "quiet" | "none" => Ok(OutputFormat::Quiet),
            other => Err(format!("unknown output format: {}", other)),
        }
    }
}

/// CLI Commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the IDE
    Start(StartArgs),
    
    /// Open files or projects
    Open(OpenArgs),
    
    /// Edit files
    Edit(EditArgs),
    
    /// View files
    View(ViewArgs),
    
    /// Diff files
    Diff(DiffArgs),
    
    /// Search in files
    Search(SearchArgs),
    
    /// Replace in files
    Replace(ReplaceArgs),
    
    /// Format code
    Format(FormatArgs),
    
    /// Lint code
    Lint(LintArgs),
    
    /// Build project
    Build(BuildArgs),
    
    /// Run project
    Run(RunArgs),
    
    /// Test project
    Test(TestArgs),
    
    /// Debug project
    Debug(DebugArgs),
    
    /// Analyze code
    Analyze(AnalyzeArgs),
    
    /// Manage extensions
    Extension(ExtensionArgs),
    
    /// Manage themes
    Theme(ThemeArgs),
    
    /// Manage configuration
    Config(ConfigArgs),
    
    /// Manage projects
    Project(ProjectArgs),
    
    /// Install packages
    Install(InstallArgs),
    
    /// Uninstall packages
    Uninstall(UninstallArgs),
    
    /// Update packages
    Update(UpdateArgs),
    
    /// List items
    List(ListArgs),
    
    /// Start server mode
    Server(ServerArgs),
    
    /// Client mode
    Client(ClientArgs),
    
    /// Watch mode
    Watch(WatchArgs),
    
    /// Migrate settings
    Migrate(MigrateArgs),
    
    /// Generate shell completions
    Completions(CompletionsArgs),
    
    /// Show version
    Version,
    
    /// Show help
    HelpCmd(HelpArgs),
}

/// Start arguments
#[derive(Args, Debug)]
pub struct StartArgs {
    /// Files to open
    pub files: Vec<PathBuf>,
    
    /// Start in server mode
    #[arg(long)]
    pub server: bool,
    
    /// Server port
    #[arg(long, default_value_t = 8080)]
    pub port: u16,
    
    /// Server host
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
}

/// Open arguments
#[derive(Args, Debug)]
pub struct OpenArgs {
    /// Files/directories to open
    pub targets: Vec<PathBuf>,
    
    /// Open in new window
    #[arg(short, long)]
    pub new_window: bool,
    
    /// Line number
    #[arg(short, long)]
    pub line: Option<usize>,
}

/// Edit arguments
#[derive(Args, Debug)]
pub struct EditArgs {
    /// Files to edit
    pub files: Vec<PathBuf>,
    
    /// Create file if it doesn't exist
    #[arg(short, long)]
    pub create: bool,
    
    /// Edit in place
    #[arg(short, long)]
    pub in_place: bool,
}

/// View arguments
#[derive(Args, Debug)]
pub struct ViewArgs {
    /// Files to view
    pub files: Vec<PathBuf>,
    
    /// View as hex
    #[arg(long)]
    pub hex: bool,
    
    /// Show line numbers
    #[arg(short, long)]
    pub line_numbers: bool,
}

/// Diff arguments
#[derive(Args, Debug)]
pub struct DiffArgs {
    /// First file
    pub left: PathBuf,
    
    /// Second file
    pub right: PathBuf,
    
    /// Unified diff format
    #[arg(short, long)]
    pub unified: bool,
}

/// Search arguments
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search pattern
    pub pattern: String,
    
    /// Paths to search
    pub paths: Vec<PathBuf>,
    
    /// Use regex
    #[arg(short, long)]
    pub regex: bool,
    
    /// Ignore case
    #[arg(short, long)]
    pub ignore_case: bool,
    
    /// Show line numbers
    #[arg(short, long)]
    pub line_numbers: bool,
}

/// Replace arguments
#[derive(Args, Debug)]
pub struct ReplaceArgs {
    /// Search pattern
    pub pattern: String,
    
    /// Replacement text
    pub replacement: String,
    
    /// Paths to process
    pub paths: Vec<PathBuf>,
    
    /// Use regex
    #[arg(short, long)]
    pub regex: bool,
    
    /// Ignore case
    #[arg(short, long)]
    pub ignore_case: bool,
    
    /// Interactive mode
    #[arg(short, long)]
    pub interactive: bool,
    
    /// Create backup
    #[arg(short, long)]
    pub backup: bool,
    
    /// Dry run
    #[arg(short, long)]
    pub dry_run: bool,
}

/// Format arguments
#[derive(Args, Debug)]
pub struct FormatArgs {
    /// Files to format
    pub files: Vec<PathBuf>,
    
    /// Check only
    #[arg(short, long)]
    pub check: bool,
}

/// Lint arguments
#[derive(Args, Debug)]
pub struct LintArgs {
    /// Files to lint
    pub files: Vec<PathBuf>,
    
    /// Fix issues
    #[arg(short, long)]
    pub fix: bool,
}

/// Build arguments
#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Build target
    #[arg(short, long)]
    pub target: Option<String>,
    
    /// Release mode
    #[arg(short, long)]
    pub release: bool,
}

/// Run arguments
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Run target
    pub target: Option<String>,
    
    /// Arguments to pass
    pub args: Vec<String>,
}

/// Test arguments
#[derive(Args, Debug)]
pub struct TestArgs {
    /// Test filter
    pub filter: Option<String>,
    
    /// Show output
    #[arg(short, long)]
    pub nocapture: bool,
}

/// Debug arguments
#[derive(Args, Debug)]
pub struct DebugArgs {
    /// Debug target
    pub target: Option<String>,
}

/// Analyze arguments
#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    /// Analysis target
    pub target: Option<String>,
}

/// Extension arguments
#[derive(Args, Debug)]
pub struct ExtensionArgs {
    #[command(subcommand)]
    pub command: ExtensionCommands,
}

#[derive(Subcommand, Debug)]
pub enum ExtensionCommands {
    /// Install extension
    Install { id: String, version: Option<String> },
    /// Uninstall extension
    Uninstall { id: String },
    /// Update extension
    Update { id: Option<String> },
    /// List extensions
    List,
    /// Enable extension
    Enable { id: String },
    /// Disable extension
    Disable { id: String },
}

/// Theme arguments
#[derive(Args, Debug)]
pub struct ThemeArgs {
    #[command(subcommand)]
    pub command: ThemeCommands,
}

#[derive(Subcommand, Debug)]
pub enum ThemeCommands {
    /// List themes
    List,
    /// Install theme
    Install { id: String },
    /// Apply theme
    Apply { name: String },
    /// Create theme
    Create { name: String },
}

/// Config arguments
#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Get config value
    Get { key: String },
    /// Set config value
    Set { key: String, value: String },
    /// List config
    List,
    /// Edit config
    Edit,
}

/// Project arguments
#[derive(Args, Debug)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommands,
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// Create project
    Create { name: String, template: Option<String> },
    /// Open project
    Open { name: String },
    /// List projects
    List,
    /// Close project
    Close,
}

/// Install arguments
#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Package to install
    pub package: String,
    
    /// Version
    #[arg(short, long)]
    pub version: Option<String>,
    
    /// Global install
    #[arg(short, long)]
    pub global: bool,
}

/// Uninstall arguments
#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// Package to uninstall
    pub package: String,
    
    /// Global uninstall
    #[arg(short, long)]
    pub global: bool,
}

/// Update arguments
#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Packages to update
    pub packages: Vec<String>,
    
    /// Check only
    #[arg(short, long)]
    pub check: bool,
}

/// List arguments
#[derive(Args, Debug)]
pub struct ListArgs {
    /// What to list
    pub what: String,
    
    /// Output format
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,
}

/// Server arguments
#[derive(Args, Debug)]
pub struct ServerArgs {
    /// Server port
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
    
    /// Server host
    #[arg(long, default_value_t = String::from("0.0.0.0"))]
    pub host: String,
    
    /// Run as daemon
    #[arg(short, long)]
    pub daemon: bool,
    
    /// PID file
    #[arg(long)]
    pub pid_file: Option<PathBuf>,
}

/// Client arguments
#[derive(Args, Debug)]
pub struct ClientArgs {
    /// Server URL
    pub url: String,
    
    /// Command to execute
    pub command: String,
    
    /// Command arguments
    pub args: Vec<String>,
}

/// Watch arguments
#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Paths to watch
    pub paths: Vec<PathBuf>,
    
    /// Command to run
    pub command: String,
    
    /// Command arguments
    pub args: Vec<String>,
    
    /// Debounce milliseconds
    #[arg(short, long, default_value_t = 100)]
    pub debounce: u64,
}

/// Migrate arguments
#[derive(Args, Debug)]
pub struct MigrateArgs {
    /// Migration type
    pub migration_type: String,
    
    /// Dry run
    #[arg(short, long)]
    pub dry_run: bool,
}

/// Completions arguments
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell type
    pub shell: String,
    
    /// Output file
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Help arguments
#[derive(Args, Debug)]
pub struct HelpArgs {
    /// Command to get help for
    pub command: Option<String>,
}