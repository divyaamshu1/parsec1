import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { File, Terminal, Theme, Keybinding } from '../types'

interface AppState {
  // State
  files: Map<string, File>
  activeFile: string | null
  terminals: Map<string, Terminal>
  activeTerminal: string | null
  theme: 'dark' | 'light'
  sidebarOpen: boolean
  terminalHeight: number
  status: 'ready' | 'busy' | 'error'
  error: string | null
  modalOpen: 'none' | 'command' | 'settings' | 'quickopen'
  keybindings: Keybinding[]
  workspacePath: string | null

  // Actions
  init: () => Promise<void>
  openFile: (path: string) => Promise<void>
  saveFile: () => Promise<void>
  saveFileAs: (path: string) => Promise<void>
  closeFile: (path: string) => void
  updateFileContent: (path: string, content: string) => void
  
  createTerminal: () => Promise<void>
  closeTerminal: (id: string) => void
  writeToTerminal: (id: string, data: string) => void
  clearTerminal: (id: string) => void
  
  toggleTheme: () => void
  setTheme: (theme: 'dark' | 'light') => void
  
  toggleSidebar: () => void
  setSidebarOpen: (open: boolean) => void
  
  setTerminalHeight: (height: number) => void
  setStatus: (status: 'ready' | 'busy' | 'error', error?: string) => void
  
  openModal: (modal: 'command' | 'settings' | 'quickopen') => void
  closeModal: () => void
  
  setWorkspace: (path: string) => Promise<void>
  getWorkspaceFiles: () => Promise<any[]>
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      files: new Map(),
      activeFile: null,
      terminals: new Map(),
      activeTerminal: null,
      theme: 'dark',
      sidebarOpen: true,
      terminalHeight: 200,
      status: 'ready',
      error: null,
      modalOpen: 'none',
      keybindings: [],
      workspacePath: null,

      init: async () => {
        try {
          // Listen for file changes from backend
          await listen('file-changed', (event: any) => {
            const { path, content } = event.payload
            const files = new Map(get().files)
            const file = files.get(path)
            if (file) {
              files.set(path, { ...file, content, dirty: false })
              set({ files })
            }
          })

          // Listen for terminal output
          await listen('terminal-output', (event: any) => {
            const { id, data } = event.payload
            const terminals = new Map(get().terminals)
            const term = terminals.get(id)
            if (term) {
              terminals.set(id, { ...term, content: term.content + data })
              set({ terminals })
            }
          })

          // Listen for errors
          await listen('error', (event: any) => {
            set({ status: 'error', error: event.payload.message })
          })

          // Load keybindings
          const keybindings = await invoke('get_keybindings') as Keybinding[]
          set({ keybindings })

        } catch (error) {
          console.error('Init failed:', error)
          set({ status: 'error', error: String(error) })
        }
      },

      openFile: async (path: string) => {
        try {
          set({ status: 'busy' })
          const content = await invoke('open_file', { path }) as string
          const files = new Map(get().files)
          const name = path.split(/[/\\]/).pop() || path
          const language = name.split('.').pop() || 'text'
          
          files.set(path, {
            path,
            name,
            content,
            language,
            dirty: false,
            created: Date.now(),
            modified: Date.now()
          })
          
          set({ 
            files, 
            activeFile: path, 
            status: 'ready' 
          })
        } catch (error) {
          set({ status: 'error', error: String(error) })
        }
      },

      saveFile: async () => {
        const { activeFile, files } = get()
        if (!activeFile) return
        
        const file = files.get(activeFile)
        if (!file || !file.dirty) return
        
        try {
          set({ status: 'busy' })
          await invoke('save_file', { path: activeFile, content: file.content })
          file.dirty = false
          file.modified = Date.now()
          set({ 
            files: new Map(files), 
            status: 'ready' 
          })
        } catch (error) {
          set({ status: 'error', error: String(error) })
        }
      },

      saveFileAs: async (path: string) => {
        const { activeFile, files } = get()
        if (!activeFile) return
        
        const file = files.get(activeFile)
        if (!file) return
        
        try {
          set({ status: 'busy' })
          await invoke('save_file', { path, content: file.content })
          
          const newFile = { ...file, path, name: path.split(/[/\\]/).pop() || path }
          const newFiles = new Map(files)
          newFiles.set(path, newFile)
          newFiles.delete(activeFile)
          
          set({ 
            files: newFiles, 
            activeFile: path,
            status: 'ready' 
          })
        } catch (error) {
          set({ status: 'error', error: String(error) })
        }
      },

      closeFile: (path: string) => {
        const { files, activeFile } = get()
        const newFiles = new Map(files)
        newFiles.delete(path)
        
        let newActive = activeFile
        if (activeFile === path) {
          newActive = newFiles.size > 0 ? Array.from(newFiles.keys())[0] : null
        }
        
        set({ files: newFiles, activeFile: newActive })
      },

      updateFileContent: (path: string, content: string) => {
        const files = new Map(get().files)
        const file = files.get(path)
        if (file) {
          files.set(path, { ...file, content, dirty: true })
          set({ files })
        }
      },

      createTerminal: async () => {
        try {
          const id = await invoke('create_terminal') as string
          const terminals = new Map(get().terminals)
          terminals.set(id, { 
            id, 
            name: `Terminal ${terminals.size + 1}`,
            content: '', 
            cwd: '/',
            process: null 
          })
          set({ 
            terminals, 
            activeTerminal: id,
            terminalHeight: 200 
          })
        } catch (error) {
          set({ status: 'error', error: String(error) })
        }
      },

      closeTerminal: (id: string) => {
        const { terminals, activeTerminal } = get()
        const newTerminals = new Map(terminals)
        newTerminals.delete(id)
        
        let newActive = activeTerminal
        if (activeTerminal === id) {
          newActive = newTerminals.size > 0 ? Array.from(newTerminals.keys())[0] : null
        }
        
        set({ 
          terminals: newTerminals, 
          activeTerminal: newActive,
          terminalHeight: newTerminals.size === 0 ? 0 : 200
        })
      },

      writeToTerminal: (id: string, data: string) => {
        invoke('write_to_terminal', { id, data })
      },

      clearTerminal: (id: string) => {
        const terminals = new Map(get().terminals)
        const term = terminals.get(id)
        if (term) {
          terminals.set(id, { ...term, content: '' })
          set({ terminals })
        }
      },

      toggleTheme: () => {
        const newTheme = get().theme === 'dark' ? 'light' : 'dark'
        set({ theme: newTheme })
      },

      setTheme: (theme) => set({ theme }),

      toggleSidebar: () => set({ sidebarOpen: !get().sidebarOpen }),
      setSidebarOpen: (open) => set({ sidebarOpen: open }),

      setTerminalHeight: (height) => set({ terminalHeight: height }),
      
      setStatus: (status, error) => set({ status, error: error || null }),

      openModal: (modal) => set({ modalOpen: modal }),
      closeModal: () => set({ modalOpen: 'none' }),

      setWorkspace: async (path: string) => {
        try {
          await invoke('set_workspace', { path })
          set({ workspacePath: path })
          await get().getWorkspaceFiles()
        } catch (error) {
          set({ status: 'error', error: String(error) })
        }
      },

      getWorkspaceFiles: async () => {
        try {
          const files = await invoke('get_workspace_files') as any[]
          return files
        } catch (error) {
          set({ status: 'error', error: String(error) })
          return []
        }
      }
    }),
    {
      name: 'parsec-storage',
      partialize: (state) => ({
        theme: state.theme,
        sidebarOpen: state.sidebarOpen,
        keybindings: state.keybindings,
        workspacePath: state.workspacePath
      })
    }
  )
)