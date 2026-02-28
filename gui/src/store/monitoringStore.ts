import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'

interface Metric {
  name: string
  value: number
  unit: string
  timestamp: number
  tags?: Record<string, string>
}

interface LogEntry {
  id: string
  level: 'trace' | 'debug' | 'info' | 'warn' | 'error' | 'fatal'
  message: string
  module: string
  file?: string
  line?: number
  timestamp: number
}

interface TraceSpan {
  id: string
  name: string
  startTime: number
  endTime?: number
  tags: Record<string, string>
  logs: Array<{ timestamp: number; message: string }>
}

interface Alert {
  id: string
  name: string
  severity: 'info' | 'warning' | 'error' | 'critical'
  message: string
  status: 'active' | 'resolved' | 'acknowledged'
  createdAt: number
  resolvedAt?: number
}

interface MonitoringState {
  metrics: Metric[]
  logs: LogEntry[]
  traces: TraceSpan[]
  alerts: Alert[]
  activeTrace: TraceSpan | null
  logFilters: {
    level?: string[]
    module?: string[]
    search?: string
  }
  timeRange: { start: number; end: number } | null
  loading: boolean
  error: string | null

  collectMetrics: () => Promise<void>
  getMetrics: (names: string[], timeRange?: { start: number; end: number }) => Promise<Metric[]>
  
  getLogs: (filters?: any, limit?: number) => Promise<LogEntry[]>
  streamLogs: (callback: (log: LogEntry) => void) => Promise<() => void>
  exportLogs: (format: 'json' | 'txt' | 'csv') => Promise<string>
  
  startTrace: (name: string, tags?: Record<string, string>) => Promise<string>
  endTrace: (id: string) => Promise<TraceSpan>
  getTrace: (id: string) => Promise<TraceSpan>
  getTraces: (name?: string, timeRange?: { start: number; end: number }) => Promise<TraceSpan[]>
  
  getAlerts: (status?: 'active' | 'resolved' | 'acknowledged') => Promise<Alert[]>
  acknowledgeAlert: (id: string) => Promise<void>
  resolveAlert: (id: string) => Promise<void>
  
  getSystemInfo: () => Promise<any>
  getProcessInfo: () => Promise<any>
  getCPUInfo: () => Promise<any>
  getMemoryInfo: () => Promise<any>
  getDiskInfo: () => Promise<any>
  getNetworkInfo: () => Promise<any>
  
  setLogFilters: (filters: any) => void
  setTimeRange: (range: { start: number; end: number } | null) => void
}

export const useMonitoringStore = create<MonitoringState>((set, get) => ({
  metrics: [],
  logs: [],
  traces: [],
  alerts: [],
  activeTrace: null,
  logFilters: {},
  timeRange: null,
  loading: false,
  error: null,

  collectMetrics: async () => {
    try {
      const metrics = await invoke('collect_metrics') as Metric[]
      set({ metrics: [...metrics, ...get().metrics].slice(0, 1000) })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  getMetrics: async (names: string[], timeRange?: { start: number; end: number }) => {
    try {
      return await invoke('get_metrics', { names, timeRange }) as Metric[]
    } catch (error) {
      set({ error: String(error) })
      return []
    }
  },

  getLogs: async (filters?: any, limit?: number) => {
    set({ loading: true })
    try {
      const logs = await invoke('get_logs', { filters, limit }) as LogEntry[]
      set({ logs, loading: false })
      return logs
    } catch (error) {
      set({ error: String(error), loading: false })
      return []
    }
  },

  streamLogs: async (callback: (log: LogEntry) => void) => {
    try {
      await invoke('stream_logs')
      const unsubscribe = await listen('log-entry', (event: any) => {
        const log = event.payload as LogEntry
        callback(log)
        set({ logs: [log, ...get().logs].slice(0, 1000) })
      })
      return unsubscribe
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  exportLogs: async (format: 'json' | 'txt' | 'csv') => {
    try {
      return await invoke('export_logs', { format }) as string
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  startTrace: async (name: string, tags?: Record<string, string>) => {
    try {
      const id = await invoke('start_trace', { name, tags }) as string
      return id
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  endTrace: async (id: string) => {
    try {
      const trace = await invoke('end_trace', { id }) as TraceSpan
      set({ traces: [trace, ...get().traces].slice(0, 100) })
      return trace
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getTrace: async (id: string) => {
    try {
      return await invoke('get_trace', { id }) as TraceSpan
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getTraces: async (name?: string, timeRange?: { start: number; end: number }) => {
    try {
      return await invoke('get_traces', { name, timeRange }) as TraceSpan[]
    } catch (error) {
      set({ error: String(error) })
      return []
    }
  },

  getAlerts: async (status?: 'active' | 'resolved' | 'acknowledged') => {
    try {
      const alerts = await invoke('get_alerts', { status }) as Alert[]
      set({ alerts })
      return alerts
    } catch (error) {
      set({ error: String(error) })
      return []
    }
  },

  acknowledgeAlert: async (id: string) => {
    try {
      await invoke('acknowledge_alert', { id })
      await get().getAlerts()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  resolveAlert: async (id: string) => {
    try {
      await invoke('resolve_alert', { id })
      await get().getAlerts()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getSystemInfo: async () => {
    try {
      return await invoke('get_system_info')
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  getProcessInfo: async () => {
    try {
      return await invoke('get_process_info')
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  getCPUInfo: async () => {
    try {
      return await invoke('get_cpu_info')
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  getMemoryInfo: async () => {
    try {
      return await invoke('get_memory_info')
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  getDiskInfo: async () => {
    try {
      return await invoke('get_disk_info')
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  getNetworkInfo: async () => {
    try {
      return await invoke('get_network_info')
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  },

  setLogFilters: (filters: any) => {
    set({ logFilters: filters })
  },

  setTimeRange: (range: { start: number; end: number } | null) => {
    set({ timeRange: range })
  }
}))