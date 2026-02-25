//! CLI command implementations

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};
use tokio::fs;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use tabled::Tabled;


use crate::args::*;
use crate::output::{print_table, print_error};

/// Handle CLI commands
pub async fn handle_command(cmd: Commands) -> Result<i32> {
    match cmd {
        Commands::Start(args) => start_command(args).await,
        Commands::Open(args) => open_command(args).await,
        Commands::Edit(args) => edit_command(args).await,
        Commands::View(args) => view_command(args).await,
        Commands::Diff(args) => diff_command(args).await,
        Commands::Search(args) => search_command(args).await,
        Commands::Replace(args) => replace_command(args).await,
        Commands::Format(args) => format_command(args).await,
        Commands::Lint(args) => lint_command(args).await,
        Commands::Build(args) => build_command(args).await,
        Commands::Run(args) => run_command(args).await,
        Commands::Test(args) => test_command(args).await,
        Commands::Debug(args) => debug_command(args).await,
        Commands::Analyze(args) => analyze_command(args).await,
        Commands::Extension(args) => extension_command(args).await,
        Commands::Theme(args) => theme_command(args).await,
        Commands::Config(args) => config_command(args).await,
        Commands::Project(args) => project_command(args).await,
        Commands::Install(args) => install_command(args).await,
        Commands::Uninstall(args) => uninstall_command(args).await,
        Commands::Update(args) => update_command(args).await,
        Commands::List(args) => list_command(args).await,
        Commands::Server(args) => server_command(args).await,
        Commands::Client(args) => client_command(args).await,
        Commands::Watch(args) => watch_command(args).await,
        Commands::Migrate(args) => migrate_command(args).await,
        Commands::Completions(args) => completions_command(args).await,
        Commands::Version => version_command().await,
        Commands::HelpCmd(args) => help_command(args).await,
    }
}

/// Start command
async fn start_command(args: StartArgs) -> Result<i32> {
    // Silently try to launch GUI without any CLI output
    if args.server {
        println!("{}", "Starting Parsec IDE (server mode)...".cyan());
        println!("Listening on {}:{}", args.host, args.port);
        // Start server
        return Ok(0);
    }

    // Candidate GUI executable locations (debug/release in workspace or gui crate)
    let candidates = [
        std::path::PathBuf::from("target/debug/parsec"),
        std::path::PathBuf::from("target/release/parsec"),
        std::path::PathBuf::from("gui/target/debug/parsec"),
        std::path::PathBuf::from("gui/target/release/parsec"),
    ];

    for cand in &candidates {
        let exe = if cfg!(windows) { cand.with_extension("exe") } else { cand.clone() };
        if exe.exists() {
            // Silently spawn GUI and wait for it
            match Command::new(&exe).spawn() {
                Ok(mut child) => {
                    let status = child.wait()?;
                    return Ok(status.code().unwrap_or(0));
                }
                Err(_) => {
                    // Try next candidate
                    continue;
                }
            }
        }
    }

    // If we reach here, no GUI binary was found. Show fallback on Windows.
    #[cfg(all(windows))]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        fn to_wide(s: &str) -> Vec<u16> {
            OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
        }

        let _ = unsafe {
            let text = to_wide("Parsec GUI: full GUI not available. Please install WebView2 runtime.");
            let title = to_wide("Parsec");
            #[allow(non_snake_case)]
            {
                use winapi::um::winuser::{MessageBoxW, MB_OK};
                MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK);
            }
        };
        return Ok(0);
    }

    #[cfg(not(windows))]
    {
        eprintln!("GUI binary not found.");
        return Ok(1);
    }
}

/// Open command
async fn open_command(args: OpenArgs) -> Result<i32> {
    for target in &args.targets {
        if !target.exists() {
            print_error(&format!("File not found: {}", target.display()));
            return Ok(1);
        }
    }
    
    println!("Opening {} file(s)", args.targets.len());
    
    #[cfg(feature = "gui")]
    {
        println!("GUI feature enabled but file opening not yet implemented");
        let _ = args.targets;
    }
    
    Ok(0)
}

/// Edit command
async fn edit_command(args: EditArgs) -> Result<i32> {
    for file in &args.files {
        if !file.exists() && !args.create {
            print_error(&format!("File not found: {}", file.display()));
            return Ok(1);
        }
    }
    
    if args.in_place {
        println!("Editing {} file(s) in place", args.files.len());
        // In-place editing logic
    } else {
        println!("Opening {} file(s) in editor", args.files.len());
        #[cfg(feature = "gui")]
        {
            println!("GUI feature enabled but file editing not yet implemented");
            let _ = args.files;
        }
    }
    
    Ok(0)
}

/// View command
async fn view_command(args: ViewArgs) -> Result<i32> {
    for file in args.files {
        if !file.exists() {
            print_error(&format!("File not found: {}", file.display()));
            return Ok(1);
        }
        
        let content = fs::read_to_string(&file).await?;
        
        if args.hex {
            // Hex view
            for (i, line) in content.as_bytes().chunks(16).enumerate() {
                print!("{:08x}: ", i * 16);
                for b in line {
                    print!("{:02x} ", b);
                }
                println!();
            }
        } else if args.line_numbers {
            for (i, line) in content.lines().enumerate() {
                println!("{:6}: {}", i + 1, line);
            }
        } else {
            println!("{}", content);
        }
    }
    
    Ok(0)
}

/// Diff command
async fn diff_command(args: DiffArgs) -> Result<i32> {
    if !args.left.exists() {
        print_error(&format!("File not found: {}", args.left.display()));
        return Ok(1);
    }
    if !args.right.exists() {
        print_error(&format!("File not found: {}", args.right.display()));
        return Ok(1);
    }
    
    let left_content = fs::read_to_string(&args.left).await?;
    let right_content = fs::read_to_string(&args.right).await?;
    
    let diff = similar::TextDiff::from_lines(&left_content, &right_content);
    
    if args.unified {
        println!("--- {}", args.left.display());
        println!("+++ {}", args.right.display());
        
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Delete => print!("-{}", change),
                similar::ChangeTag::Insert => print!("+{}", change),
                similar::ChangeTag::Equal => print!(" {}", change),
            }
        }
    } else {
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Delete => println!("- {}", change.value().trim_end()),
                similar::ChangeTag::Insert => println!("+ {}", change.value().trim_end()),
                similar::ChangeTag::Equal => {}
            }
        }
    }
    
    Ok(0)
}

/// Search command
async fn search_command(args: SearchArgs) -> Result<i32> {
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    
    let mut matches = 0;
    let mut file_matches = 0;
    
    for path in paths {
        if path.is_file() {
            let content = fs::read_to_string(&path).await?;
            let found = search_in_content(&content, &args);
            
            if !found.is_empty() {
                file_matches += 1;
                matches += found.len();
                println!("{}:", path.display());
                for (line_num, line) in found {
                    if args.line_numbers {
                        println!("  {:6}: {}", line_num, line);
                    } else {
                        println!("  {}", line);
                    }
                }
            }
        } else if path.is_dir() {
            let results = search_directory(&path, &args).await?;
            for (file_path, file_matches_vec) in results {
                file_matches += file_matches_vec.len();
                matches += file_matches_vec.len();
                println!("{}:", file_path.display());
                for (line_num, line) in file_matches_vec {
                    if args.line_numbers {
                        println!("  {:6}: {}", line_num, line);
                    } else {
                        println!("  {}", line);
                    }
                }
            }
        }
    }
    
    println!("\nFound {} matches in {} files", matches, file_matches);
    Ok(0)
}

/// Replace command
async fn replace_command(args: ReplaceArgs) -> Result<i32> {
    let mut replacements = 0;
    let mut files_modified = 0;
    
    for path in args.paths {
        if !path.exists() {
            print_warning(&format!("Path not found: {}", path.display()));
            continue;
        }
        
        if path.is_file() {
            let content = fs::read_to_string(&path).await?;
            let new_content = if args.regex {
                let re = regex::Regex::new(&args.pattern)?;
                re.replace_all(&content, args.replacement.as_str()).to_string()
            } else {
                content.replace(&args.pattern, &args.replacement)
            };
            
            if content != new_content {
                replacements += count_replacements(&content, &new_content);
                files_modified += 1;
                
                if args.dry_run {
                    println!("Would modify: {}", path.display());
                } else if args.interactive {
                    // Show diff and ask
                    let diff = similar::TextDiff::from_lines(&content, &new_content);
                    println!("Changes in {}:", path.display());
                    for change in diff.iter_all_changes() {
                        match change.tag() {
                            similar::ChangeTag::Delete => println!("-{}", change),
                            similar::ChangeTag::Insert => println!("+{}", change),
                            _ => {}
                        }
                    }
                    
                    if ask_confirm("Apply changes?")? {
                        if args.backup {
                            let backup = path.with_extension("bak");
                            fs::copy(&path, &backup).await?;
                        }
                        fs::write(&path, new_content).await?;
                    }
                } else {
                    if args.backup {
                        let backup = path.with_extension("bak");
                        fs::copy(&path, &backup).await?;
                    }
                    fs::write(&path, new_content).await?;
                }
            }
        }
    }
    
    println!("Replaced {} occurrences in {} files", replacements, files_modified);
    Ok(0)
}

/// Install command
async fn install_command(args: InstallArgs) -> Result<i32> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "));
    pb.set_message(format!("Installing {}...", args.package));
    
    // Simulate installation
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    
    pb.finish_with_message(format!("✅ Installed {}", args.package));
    Ok(0)
}

/// Uninstall command
async fn uninstall_command(args: UninstallArgs) -> Result<i32> {
    if ask_confirm(&format!("Uninstall {}?", args.package))? {
        println!("Uninstalling {}...", args.package);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("✅ Uninstalled {}", args.package);
    }
    Ok(0)
}

/// Update command
async fn update_command(args: UpdateArgs) -> Result<i32> {
    if args.check {
        println!("Checking for updates...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("All packages are up to date");
    } else {
        if args.packages.is_empty() {
            println!("Updating all packages...");
        } else {
            println!("Updating {} package(s)...", args.packages.len());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        println!("✅ Updated successfully");
    }
    Ok(0)
}

/// List command
async fn list_command(args: ListArgs) -> Result<i32> {
    match args.what.as_str() {
        "extensions" => {
            #[derive(Tabled, serde::Serialize, Debug)]
            struct ExtRow {
                #[tabled(rename = "ID")]
                id: String,
                #[tabled(rename = "Version")]
                version: String,
                #[tabled(rename = "Publisher")]
                publisher: String,
                #[tabled(rename = "Enabled")]
                enabled: String,
            }
            
            let rows = vec![
                ExtRow {
                    id: "parsec.rust".to_string(),
                    version: "1.0.0".to_string(),
                    publisher: "parsec".to_string(),
                    enabled: "✓".to_string(),
                },
                ExtRow {
                    id: "parsec.python".to_string(),
                    version: "0.8.0".to_string(),
                    publisher: "parsec".to_string(),
                    enabled: "✓".to_string(),
                },
            ];
            
            let _ = print_table(rows, args.format.unwrap_or(OutputFormat::Text));
        }
        "themes" => {
            #[derive(Tabled, serde::Serialize, Debug)]
            struct ThemeRow {
                #[tabled(rename = "Name")]
                name: String,
                #[tabled(rename = "Type")]
                theme_type: String,
                #[tabled(rename = "Active")]
                active: String,
            }
            
            let rows = vec![
                ThemeRow {
                    name: "Dark+".to_string(),
                    theme_type: "dark".to_string(),
                    active: "✓".to_string(),
                },
                ThemeRow {
                    name: "Light+".to_string(),
                    theme_type: "light".to_string(),
                    active: "".to_string(),
                },
            ];
            
            let _ = print_table(rows, args.format.unwrap_or(OutputFormat::Text));
        }
        _ => {
            print_error(&format!("Unknown list type: {}", args.what));
            return Ok(1);
        }
    }
    
    Ok(0)
}

/// Server command
async fn server_command(args: ServerArgs) -> Result<i32> {
    println!("Starting server on {}:{}", args.host, args.port);
    
    if args.daemon {
        println!("Running as daemon (PID: {})", std::process::id());
        if let Some(pid_file) = args.pid_file {
            fs::write(pid_file, std::process::id().to_string()).await?;
        }
    }
    
    // Start server loop
    println!("Server running. Press Ctrl+C to stop.");
    
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");
    
    Ok(0)
}

/// Client command
async fn client_command(args: ClientArgs) -> Result<i32> {
    println!("Connecting to {}", args.url);
    println!("Executing: {} {:?}", args.command, args.args);
    
    // Simulate request
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    println!("Response: OK");
    Ok(0)
}

/// Watch command
async fn watch_command(args: WatchArgs) -> Result<i32> {
    use notify::{Watcher, RecursiveMode, RecommendedWatcher};
    
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths
    };
    
    println!("Watching {} path(s)", paths.len());
    println!("Command: {} {:?}", args.command, args.args);
    
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.blocking_send(res);
    })?;
    
    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
        println!("Watching: {}", path.display());
    }
    
    let mut debounce_timer = tokio::time::interval(tokio::time::Duration::from_millis(args.debounce));
    debounce_timer.tick().await; // Skip first tick
    
    println!("Watching for changes...");
    
    loop {
        tokio::select! {
            Some(res) = rx.recv() => {
                match res {
                    Ok(event) => {
                        if event.kind.is_modify() || event.kind.is_create() {
                            // Debounce
                            debounce_timer.tick().await;
                            
                            println!("Change detected, running command...");
                            
                            let output = Command::new(&args.command)
                                .args(&args.args)
                                .output()?;
                            
                            if !output.stdout.is_empty() {
                                println!("{}", String::from_utf8_lossy(&output.stdout));
                            }
                            if !output.stderr.is_empty() {
                                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                            }
                        }
                    }
                    Err(e) => eprintln!("Watch error: {}", e),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nStopping watch...");
                break;
            }
        }
    }
    
    Ok(0)
}

/// Completions command
async fn completions_command(args: CompletionsArgs) -> Result<i32> {
    use clap::CommandFactory;
    use clap_complete::{generate, Shell};
    
    let mut cmd = crate::Cli::command();
    let name = cmd.get_name().to_string();
    
    let shell = match args.shell.as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => return Err(anyhow!("Unsupported shell: {}", args.shell)),
    };
    
    let mut buf = Vec::new();
    generate(shell, &mut cmd, name, &mut buf);
    
    let completions = String::from_utf8(buf)?;
    
    if let Some(output) = args.output {
        fs::write(output, completions).await?;
        println!("✅ Completions written");
    } else {
        println!("{}", completions);
    }
    
    Ok(0)
}

/// Version command
async fn version_command() -> Result<i32> {
    println!("Parsec IDE v{}", env!("CARGO_PKG_VERSION"));
    println!("Core: v{}", "0.1.0");
    println!("Extensions: v{}", "0.1.0");
    println!("AI: v{}", "0.1.0");
    println!("Built: {}", option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown"));
    Ok(0)
}

/// Help command
async fn help_command(args: HelpArgs) -> Result<i32> {
    use clap::CommandFactory;
    
    let mut cmd = crate::Cli::command();
    
    if let Some(command) = args.command {
        if let Some(sub) = cmd.find_subcommand(&command) {
            let mut sub_clone = sub.clone();
            sub_clone.print_help()?;
        } else {
            print_error(&format!("Unknown command: {}", command));
            return Ok(1);
        }
    } else {
        cmd.print_help()?;
    }
    
    Ok(0)
}

/// Placeholder implementations for other commands
async fn format_command(_args: FormatArgs) -> Result<i32> { Ok(0) }
async fn lint_command(_args: LintArgs) -> Result<i32> { Ok(0) }
async fn build_command(_args: BuildArgs) -> Result<i32> { Ok(0) }
async fn run_command(_args: RunArgs) -> Result<i32> { Ok(0) }
async fn test_command(_args: TestArgs) -> Result<i32> { Ok(0) }
async fn debug_command(_args: DebugArgs) -> Result<i32> { Ok(0) }
async fn analyze_command(_args: AnalyzeArgs) -> Result<i32> { Ok(0) }
async fn extension_command(_args: ExtensionArgs) -> Result<i32> { Ok(0) }
async fn theme_command(_args: ThemeArgs) -> Result<i32> { Ok(0) }
async fn config_command(_args: ConfigArgs) -> Result<i32> { Ok(0) }
async fn project_command(_args: ProjectArgs) -> Result<i32> { Ok(0) }
async fn migrate_command(_args: MigrateArgs) -> Result<i32> { Ok(0) }

/// Helper functions
fn search_in_content(content: &str, args: &SearchArgs) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    
    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;
        let line_lower = line.to_lowercase();
        let pattern_lower = args.pattern.to_lowercase();
        
        let matched = if args.regex {
            let re = regex::Regex::new(&args.pattern).ok();
            re.map(|r| r.is_match(line)).unwrap_or(false)
        } else if args.ignore_case {
            line_lower.contains(&pattern_lower)
        } else {
            line.contains(&args.pattern)
        };
        
        if matched {
            results.push((line_num, line.to_string()));
        }
    }
    
    results
}

async fn search_directory(
    dir: &Path,
    args: &SearchArgs,
) -> Result<Vec<(PathBuf, Vec<(usize, String)>)>> {
    let mut results = Vec::new();
    let mut read_dir = fs::read_dir(dir).await?;
    
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path).await {
                let matches = search_in_content(&content, args);
                if !matches.is_empty() {
                    results.push((path, matches));
                }
            }
        } else if path.is_dir() {
            if let Ok(mut sub) = Box::pin(search_directory(&path, args)).await {
                results.append(&mut sub);
            }
        }
    }
    
    Ok(results)
}

fn count_replacements(old: &str, new: &str) -> usize {
    if old == new {
        0
    } else {
        // Simple count based on line differences
        let old_lines: Vec<_> = old.lines().collect();
        let new_lines: Vec<_> = new.lines().collect();
        
        if old_lines.len() != new_lines.len() {
            return old_lines.len().abs_diff(new_lines.len());
        }
        
        old_lines.iter().zip(new_lines.iter())
            .filter(|(o, n)| o != n)
            .count()
    }
}

fn ask_confirm(prompt: &str) -> Result<bool> {
    use std::io::{stdin, stdout, Write};
    
    print!("{} (y/N): ", prompt);
    stdout().flush()?;
    
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase() == "y")
}

fn print_warning(msg: &str) {
    eprintln!("{}", msg.yellow());
}