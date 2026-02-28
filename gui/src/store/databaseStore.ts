import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'
import { DatabaseConnection } from '../types'

interface QueryResult {
  columns: string[]
  rows: any[][]
  affected?: number
  duration: number
}

interface TableInfo {
  name: string
  columns: Array<{ name: string; type: string; nullable: boolean }>
  rowCount?: number
}

interface DatabaseState {
  connections: DatabaseConnection[]
  activeConnection: string | null
  databases: string[]
  tables: TableInfo[]
  queryHistory: Array<{ query: string; timestamp: number; result?: QueryResult }>
  loading: boolean
  error: string | null

  loadConnections: () => Promise<void>
  addConnection: (conn: Omit<DatabaseConnection, 'id' | 'connected'>) => Promise<void>
  removeConnection: (id: string) => Promise<void>
  connect: (id: string) => Promise<void>
  disconnect: (id: string) => Promise<void>
  
  setActiveConnection: (id: string | null) => Promise<void>
  
  getDatabases: () => Promise<string[]>
  getTables: (database: string) => Promise<TableInfo[]>
  getTableSchema: (table: string) => Promise<any>
  
  executeQuery: (query: string) => Promise<QueryResult>
  executeQueryOnConnection: (id: string, query: string) => Promise<QueryResult>
  
  exportResults: (format: 'json' | 'csv' | 'sql', data: any) => Promise<string>
  clearHistory: () => void
}

export const useDatabaseStore = create<DatabaseState>((set, get) => ({
  connections: [],
  activeConnection: null,
  databases: [],
  tables: [],
  queryHistory: [],
  loading: false,
  error: null,

  loadConnections: async () => {
    try {
      const connections = await invoke('get_db_connections') as DatabaseConnection[]
      set({ connections })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  addConnection: async (conn) => {
    set({ loading: true })
    try {
      const id = await invoke('add_db_connection', conn) as string
      await get().loadConnections()
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  removeConnection: async (id: string) => {
    set({ loading: true })
    try {
      await invoke('remove_db_connection', { id })
      await get().loadConnections()
      if (get().activeConnection === id) {
        set({ activeConnection: null, databases: [], tables: [] })
      }
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  connect: async (id: string) => {
    set({ loading: true })
    try {
      await invoke('connect_db', { id })
      const connections = get().connections.map(c => 
        c.id === id ? { ...c, connected: true } : c
      )
      set({ connections })
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  disconnect: async (id: string) => {
    set({ loading: true })
    try {
      await invoke('disconnect_db', { id })
      const connections = get().connections.map(c => 
        c.id === id ? { ...c, connected: false } : c
      )
      set({ connections })
      if (get().activeConnection === id) {
        set({ activeConnection: null, databases: [], tables: [] })
      }
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  setActiveConnection: async (id: string | null) => {
    set({ activeConnection: id, databases: [], tables: [] })
    if (id) {
      await get().getDatabases()
    }
  },

  getDatabases: async () => {
    const { activeConnection } = get()
    if (!activeConnection) return []

    try {
      const databases = await invoke('get_databases', { id: activeConnection }) as string[]
      set({ databases })
      return databases
    } catch (error) {
      set({ error: String(error) })
      return []
    }
  },

  getTables: async (database: string) => {
    const { activeConnection } = get()
    if (!activeConnection) return []

    try {
      const tables = await invoke('get_tables', { id: activeConnection, database }) as TableInfo[]
      set({ tables })
      return tables
    } catch (error) {
      set({ error: String(error) })
      return []
    }
  },

  getTableSchema: async (table: string) => {
    const { activeConnection } = get()
    if (!activeConnection) return null

    try {
      return await invoke('get_table_schema', { id: activeConnection, table })
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  executeQuery: async (query: string) => {
    const { activeConnection } = get()
    if (!activeConnection) throw new Error('No connection selected')

    return get().executeQueryOnConnection(activeConnection, query)
  },

  executeQueryOnConnection: async (id: string, query: string) => {
    set({ loading: true })
    const start = Date.now()

    try {
      const result = await invoke('execute_query', { id, query }) as QueryResult
      result.duration = Date.now() - start

      set({ 
        queryHistory: [{ query, timestamp: start, result }, ...get().queryHistory].slice(0, 100),
        loading: false 
      })
      
      return result
    } catch (error) {
      set({ 
        queryHistory: [{ query, timestamp: start }, ...get().queryHistory].slice(0, 100),
        error: String(error),
        loading: false 
      })
      throw error
    }
  },

  exportResults: async (format: 'json' | 'csv' | 'sql', data: any) => {
    try {
      return await invoke('export_query_results', { format, data }) as string
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  clearHistory: () => {
    set({ queryHistory: [] })
  }
}))