#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, Window, AppHandle, CustomMenuItem, Menu, MenuItem, Submenu};
use std::sync::Mutex;
use tracing::{info, error, debug};
use tracing_subscriber;

// Import all parsec backend crates
use parsec_core::editor::Editor;
use parsec_terminal::{Multiplexer, TerminalConfig};
use parsec_ai::AIEngine;
use parsec_cloud::CloudManager;
use parsec_database::DatabaseManager;
use parsec_mobile::MobileManager;
use parsec_collaboration::CollaborationEngine;
use parsec_learning::LearningEngine;
use parsec_design::DesignEngine;
use parsec_monitoring::MonitoringEngine;
use parsec_accessibility::AccessibilityEngine;
use parsec_customization::CustomizationEngine;
use parsec_debug::Debugger;
use parsec_git::GitManager;
use parsec_testing::TestRunner;
use parsec_api::APIClient;

// App state
struct AppState {
    editor: Mutex<Editor>,
    terminal: Mutex<Multiplexer>,
    ai: Mutex<AIEngine>,
    cloud: Mutex<CloudManager>,
    database: Mutex<DatabaseManager>,
    mobile: Mutex<MobileManager>,
    collaboration: Mutex<CollaborationEngine>,
    learning: Mutex<LearningEngine>,
    design: Mutex<DesignEngine>,
    monitoring: Mutex<MonitoringEngine>,
    accessibility: Mutex<AccessibilityEngine>,
    customization: Mutex<CustomizationEngine>,
    debugger: Mutex<Debugger>,
    git: Mutex<GitManager>,
    testing: Mutex<TestRunner>,
    api: Mutex<APIClient>,
}

// ==================== Editor Commands ====================

#[tauri::command]
async fn open_file(state: tauri::State<'_, AppState>, path: String) -> Result<String, String> {
    let mut editor = state.editor.lock().unwrap();
    editor.open_file(&path).await.map_err(|e| e.to_string())?;
    let content = editor.get_content();
    Ok(content)
}

#[tauri::command]
async fn save_file(state: tauri::State<'_, AppState>, path: String, content: String) -> Result<(), String> {
    let mut editor = state.editor.lock().unwrap();
    editor.save_file(&path, &content).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_workspace_files(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let editor = state.editor.lock().unwrap();
    let files = editor.get_workspace_files().await.map_err(|e| e.to_string())?;
    Ok(files)
}

#[tauri::command]
async fn set_workspace(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let mut editor = state.editor.lock().unwrap();
    editor.set_workspace(&path).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Terminal Commands ====================

#[tauri::command]
async fn create_terminal(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mut terminal = state.terminal.lock().unwrap();
    let id = terminal.create_session(None, None).await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
async fn write_to_terminal(state: tauri::State<'_, AppState>, id: String, data: String) -> Result<(), String> {
    let terminal = state.terminal.lock().unwrap();
    let session = terminal.get_session(&id).ok_or("Terminal not found")?;
    session.write(data.as_bytes()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn resize_terminal(state: tauri::State<'_, AppState>, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let terminal = state.terminal.lock().unwrap();
    let session = terminal.get_session(&id).ok_or("Terminal not found")?;
    session.resize(rows, cols).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== AI Commands ====================

#[tauri::command]
async fn get_ai_providers(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let ai = state.ai.lock().unwrap();
    let providers = ai.list_providers().await;
    Ok(providers)
}

#[tauri::command]
async fn set_active_ai_provider(state: tauri::State<'_, AppState>, provider: String) -> Result<(), String> {
    let mut ai = state.ai.lock().unwrap();
    ai.set_active_provider(&provider).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn ai_complete(
    state: tauri::State<'_, AppState>,
    provider: String,
    model: String,
    prompt: String,
) -> Result<String, String> {
    let ai = state.ai.lock().unwrap();
    let response = ai.complete(prompt).await.map_err(|e| e.to_string())?;
    Ok(response)
}

// ==================== Cloud Commands ====================

#[tauri::command]
async fn list_cloud_services(state: tauri::State<'_, AppState>, provider: String) -> Result<Vec<String>, String> {
    let cloud = state.cloud.lock().unwrap();
    let services = cloud.list_services(&provider).await.map_err(|e| e.to_string())?;
    Ok(services)
}

#[tauri::command]
async fn deploy_cloud_function(
    state: tauri::State<'_, AppState>,
    provider: String,
    name: String,
    runtime: String,
    code: String,
) -> Result<(), String> {
    let cloud = state.cloud.lock().unwrap();
    cloud.deploy_function(&provider, &name, &runtime, &code).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Database Commands ====================

#[tauri::command]
async fn get_db_connections(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = state.database.lock().unwrap();
    let connections = db.list_connections().await.map_err(|e| e.to_string())?;
    Ok(connections)
}

#[tauri::command]
async fn add_db_connection(
    state: tauri::State<'_, AppState>,
    name: String,
    db_type: String,
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
) -> Result<String, String> {
    let mut db = state.database.lock().unwrap();
    let id = db.add_connection(&name, &db_type, &host, port, &database, &username, &password)
        .await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
async fn execute_query(state: tauri::State<'_, AppState>, id: String, query: String) -> Result<String, String> {
    let db = state.database.lock().unwrap();
    let result = db.execute_query(&id, &query).await.map_err(|e| e.to_string())?;
    Ok(result)
}

// ==================== Git Commands ====================

#[tauri::command]
async fn get_branches(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let git = state.git.lock().unwrap();
    let branches = git.get_branches().await.map_err(|e| e.to_string())?;
    Ok(branches)
}

#[tauri::command]
async fn checkout_branch(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    let mut git = state.git.lock().unwrap();
    git.checkout_branch(&name).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn commit(state: tauri::State<'_, AppState>, message: String) -> Result<(), String> {
    let mut git = state.git.lock().unwrap();
    git.commit(&message).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Debug Commands ====================

#[tauri::command]
async fn start_debugging(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut debugger = state.debugger.lock().unwrap();
    debugger.start().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn set_breakpoint(state: tauri::State<'_, AppState>, file: String, line: u32) -> Result<(), String> {
    let mut debugger = state.debugger.lock().unwrap();
    debugger.set_breakpoint(&file, line).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_stack_trace(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let debugger = state.debugger.lock().unwrap();
    let trace = debugger.get_stack_trace().await.map_err(|e| e.to_string())?;
    Ok(trace)
}

// ==================== Extension Commands ====================

#[tauri::command]
async fn get_extensions(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let ext = state.extensions.lock().unwrap();
    let extensions = ext.list_extensions().await;
    Ok(extensions)
}

#[tauri::command]
async fn install_extension(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let mut ext = state.extensions.lock().unwrap();
    ext.install_extension(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Theme Commands ====================

#[tauri::command]
async fn get_themes(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let customization = state.customization.lock().unwrap();
    let themes = customization.get_themes().await;
    Ok(themes)
}

#[tauri::command]
async fn set_theme(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let mut customization = state.customization.lock().unwrap();
    customization.set_active_theme(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Initialize App ====================

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting Parsec IDE...");

    // Build menu
    let menu = Menu::new()
        .add_submenu(Submenu::new("File", Menu::new()
            .add_item(CustomMenuItem::new("new_file".to_string(), "New File"))
            .add_item(CustomMenuItem::new("open".to_string(), "Open..."))
            .add_item(CustomMenuItem::new("save".to_string(), "Save"))
            .add_item(CustomMenuItem::new("save_as".to_string(), "Save As..."))
            .add_native_item(MenuItem::Separator)
            .add_item(CustomMenuItem::new("close".to_string(), "Close"))
            .add_item(CustomMenuItem::new("close_all".to_string(), "Close All"))
        ))
        .add_submenu(Submenu::new("Edit", Menu::new()
            .add_item(CustomMenuItem::new("undo".to_string(), "Undo"))
            .add_item(CustomMenuItem::new("redo".to_string(), "Redo"))
            .add_native_item(MenuItem::Separator)
            .add_item(CustomMenuItem::new("cut".to_string(), "Cut"))
            .add_item(CustomMenuItem::new("copy".to_string(), "Copy"))
            .add_item(CustomMenuItem::new("paste".to_string(), "Paste"))
        ))
        .add_submenu(Submenu::new("View", Menu::new()
            .add_item(CustomMenuItem::new("toggle_sidebar".to_string(), "Toggle Sidebar"))
            .add_item(CustomMenuItem::new("toggle_terminal".to_string(), "Toggle Terminal"))
            .add_native_item(MenuItem::Separator)
            .add_item(CustomMenuItem::new("zoom_in".to_string(), "Zoom In"))
            .add_item(CustomMenuItem::new("zoom_out".to_string(), "Zoom Out"))
        ))
        .add_submenu(Submenu::new("Help", Menu::new()
            .add_item(CustomMenuItem::new("documentation".to_string(), "Documentation"))
            .add_item(CustomMenuItem::new("about".to_string(), "About Parsec"))
        ));

    tauri::Builder::default()
        .menu(menu)
        .on_menu_event(|event| match event.menu_item_id() {
            "new_file" => {
                event.window().emit("menu-new-file", {}).unwrap();
            }
            "open" => {
                event.window().emit("menu-open", {}).unwrap();
            }
            "save" => {
                event.window().emit("menu-save", {}).unwrap();
            }
            "toggle_sidebar" => {
                event.window().emit("menu-toggle-sidebar", {}).unwrap();
            }
            "toggle_terminal" => {
                event.window().emit("menu-toggle-terminal", {}).unwrap();
            }
            "about" => {
                event.window().emit("menu-about", {}).unwrap();
            }
            _ => {}
        })
        .manage(AppState {
            editor: Mutex::new(Editor::new()),
            terminal: Mutex::new(Multiplexer::new(TerminalConfig::default())),
            ai: Mutex::new(AIEngine::new(parsec_ai::AIConfig::default())),
            cloud: Mutex::new(CloudManager::new()),
            database: Mutex::new(DatabaseManager::new()),
            mobile: Mutex::new(MobileManager::new()),
            collaboration: Mutex::new(CollaborationEngine::new(
                parsec_collaboration::CollaborationConfig::default(),
                parsec_collaboration::UserId::new(),
            )),
            learning: Mutex::new(LearningEngine::new(
                parsec_learning::LearningConfig::default(),
                None,
            )),
            design: Mutex::new(DesignEngine::new(parsec_design::DesignConfig::default())),
            monitoring: Mutex::new(MonitoringEngine::new(parsec_monitoring::MonitoringConfig::default())),
            accessibility: Mutex::new(AccessibilityEngine::new(parsec_accessibility::AccessibilityConfig::default())),
            customization: Mutex::new(CustomizationEngine::new(parsec_customization::CustomizationConfig::default())),
            debugger: Mutex::new(Debugger::new()),
            git: Mutex::new(GitManager::new(parsec_git::GitConfig::default())),
            testing: Mutex::new(TestRunner::new()),
            api: Mutex::new(APIClient::new()),
        })
        .invoke_handler(tauri::generate_handler![
            open_file,
            save_file,
            get_workspace_files,
            set_workspace,
            create_terminal,
            write_to_terminal,
            resize_terminal,
            get_ai_providers,
            set_active_ai_provider,
            ai_complete,
            list_cloud_services,
            deploy_cloud_function,
            get_db_connections,
            add_db_connection,
            execute_query,
            get_branches,
            checkout_branch,
            commit,
            start_debugging,
            set_breakpoint,
            get_stack_trace,
            get_extensions,
            install_extension,
            get_themes,
            set_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}