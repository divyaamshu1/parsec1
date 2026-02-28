import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { appWindow } from '@tauri-apps/api/window'
import { fs } from '@tauri-apps/api'
import { dialog } from '@tauri-apps/api'
import { shell } from '@tauri-apps/api'

// ==================== File System ====================

export async function readFile(path: string): Promise<string> {
  return await invoke('read_file', { path })
}

export async function writeFile(path: string, content: string): Promise<void> {
  await invoke('write_file', { path, content })
}

export async function deleteFile(path: string): Promise<void> {
  await invoke('delete_file', { path })
}

export async function readDir(path: string): Promise<any[]> {
  return await invoke('read_dir', { path })
}

export async function createDir(path: string): Promise<void> {
  await invoke('create_dir', { path })
}

export async function exists(path: string): Promise<boolean> {
  return await invoke('exists', { path })
}

// ==================== Window ====================

export async function setTitle(title: string): Promise<void> {
  await appWindow.setTitle(title)
}

export async function maximize(): Promise<void> {
  await appWindow.maximize()
}

export async function minimize(): Promise<void> {
  await appWindow.minimize()
}

export async function close(): Promise<void> {
  await appWindow.close()
}

export async function isFullscreen(): Promise<boolean> {
  return await appWindow.isFullscreen()
}

// ==================== Dialogs ====================

export async function openFileDialog(options?: any): Promise<string | null> {
  const result = await dialog.open({
    multiple: false,
    ...options
  })
  return result as string | null
}

export async function saveFileDialog(options?: any): Promise<string | null> {
  const result = await dialog.save({
    ...options
  })
  return result as string | null
}

export async function showMessageDialog(message: string, options?: any): Promise<void> {
  await dialog.message(message, options)
}

export async function showConfirmDialog(message: string): Promise<boolean> {
  const result = await dialog.ask(message, { title: 'Confirm', type: 'warning' })
  return result
}

// ==================== Shell ====================

export async function openInBrowser(url: string): Promise<void> {
  await shell.open(url)
}

export async function openInExplorer(path: string): Promise<void> {
  await shell.open(path)
}

// ==================== Events ====================

export async function onFileChanged(callback: (path: string, content: string) => void): Promise<() => void> {
  const unlisten = await listen('file-changed', (event: any) => {
    callback(event.payload.path, event.payload.content)
  })
  return unlisten
}

export async function onTerminalOutput(callback: (id: string, data: string) => void): Promise<() => void> {
  const unlisten = await listen('terminal-output', (event: any) => {
    callback(event.payload.id, event.payload.data)
  })
  return unlisten
}

export async function onError(callback: (error: string) => void): Promise<() => void> {
  const unlisten = await listen('error', (event: any) => {
    callback(event.payload.message)
  })
  return unlisten
}

// ==================== Editor Commands ====================

export async function openFile(path: string): Promise<string> {
  return await invoke('open_file', { path })
}

export async function saveFile(path: string, content: string): Promise<void> {
  await invoke('save_file', { path, content })
}

export async function getWorkspaceFiles(): Promise<any[]> {
  return await invoke('get_workspace_files')
}

export async function setWorkspace(path: string): Promise<void> {
  await invoke('set_workspace', { path })
}

// ==================== Terminal Commands ====================

export async function createTerminal(): Promise<string> {
  return await invoke('create_terminal')
}

export async function writeToTerminal(id: string, data: string): Promise<void> {
  await invoke('write_to_terminal', { id, data })
}

export async function resizeTerminal(id: string, cols: number, rows: number): Promise<void> {
  await invoke('resize_terminal', { id, cols, rows })
}

// ==================== AI Commands ====================

export async function getAIProviders(): Promise<any[]> {
  return await invoke('get_ai_providers')
}

export async function setActiveAIProvider(provider: string): Promise<void> {
  await invoke('set_active_ai_provider', { provider })
}

export async function aiComplete(provider: string, model: string, prompt: string, options?: any): Promise<string> {
  return await invoke('ai_complete', { provider, model, prompt, options })
}

export async function aiCompletions(provider: string, model: string, prompt: string, n: number): Promise<string[]> {
  return await invoke('ai_completions', { provider, model, prompt, n })
}

export async function aiChat(provider: string, model: string, messages: any[]): Promise<string> {
  return await invoke('ai_chat', { provider, model, messages })
}

// ==================== Cloud Commands ====================

export async function listCloudServices(provider: string): Promise<any[]> {
  return await invoke('list_cloud_services', { provider })
}

export async function deployCloudFunction(provider: string, name: string, runtime: string, code: string): Promise<void> {
  await invoke('deploy_cloud_function', { provider, name, runtime, code })
}

export async function startCloudService(provider: string, id: string): Promise<void> {
  await invoke('start_cloud_service', { provider, id })
}

export async function stopCloudService(provider: string, id: string): Promise<void> {
  await invoke('stop_cloud_service', { provider, id })
}

// ==================== Database Commands ====================

export async function getDBConnections(): Promise<any[]> {
  return await invoke('get_db_connections')
}

export async function addDBConnection(connection: any): Promise<string> {
  return await invoke('add_db_connection', connection)
}

export async function connectDB(id: string): Promise<void> {
  await invoke('connect_db', { id })
}

export async function disconnectDB(id: string): Promise<void> {
  await invoke('disconnect_db', { id })
}

export async function executeQuery(id: string, query: string): Promise<any> {
  return await invoke('execute_query', { id, query })
}

// ==================== Git Commands ====================

export async function getBranches(): Promise<any[]> {
  return await invoke('get_branches')
}

export async function checkoutBranch(name: string): Promise<void> {
  await invoke('checkout_branch', { name })
}

export async function createBranch(name: string, from: string): Promise<void> {
  await invoke('create_branch', { name, from })
}

export async function getCommits(): Promise<any[]> {
  return await invoke('get_commits')
}

export async function stageFile(path: string): Promise<void> {
  await invoke('stage_file', { path })
}

export async function commit(message: string): Promise<void> {
  await invoke('commit', { message })
}

// ==================== Debug Commands ====================

export async function startDebugging(): Promise<void> {
  await invoke('start_debugging')
}

export async function stopDebugging(): Promise<void> {
  await invoke('stop_debugging')
}

export async function stepOver(): Promise<void> {
  await invoke('step_over')
}

export async function stepInto(): Promise<void> {
  await invoke('step_into')
}

export async function stepOut(): Promise<void> {
  await invoke('step_out')
}

export async function setBreakpoint(file: string, line: number): Promise<void> {
  await invoke('set_breakpoint', { file, line })
}

export async function getStackTrace(): Promise<any[]> {
  return await invoke('get_stack_trace')
}

// ==================== Extension Commands ====================

export async function getExtensions(): Promise<any[]> {
  return await invoke('get_extensions')
}

export async function installExtension(id: string): Promise<void> {
  await invoke('install_extension', { id })
}

export async function uninstallExtension(id: string): Promise<void> {
  await invoke('uninstall_extension', { id })
}

export async function enableExtension(id: string): Promise<void> {
  await invoke('enable_extension', { id })
}

export async function disableExtension(id: string): Promise<void> {
  await invoke('disable_extension', { id })
}

// ==================== Theme Commands ====================

export async function getThemes(): Promise<any[]> {
  return await invoke('get_themes')
}

export async function setTheme(id: string): Promise<void> {
  await invoke('set_theme', { id })
}

export async function getKeybindings(): Promise<any[]> {
  return await invoke('get_keybindings')
}

export async function setKeybinding(key: string, command: string): Promise<void> {
  await invoke('set_keybinding', { key, command })
}