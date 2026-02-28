import { useState, useCallback, useEffect } from 'react'
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
  duration?: number
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
  acknowledgedBy?: string
}

interface SystemInfo {
  hostname: string
  os: string
  arch: string
  kernel: string
  uptime: number
  processes: number
  loadAverage: number[]
}

interface CPUInfo {
  cores: number
  model: string
  usage: number
  frequency: number
  temperature?: number
  load: number[]
}

interface MemoryInfo {
  total: number
  used: number
  free: number
  available: number
  swapTotal: number
  swapUsed: number
  swapFree: number
}

interface DiskInfo {
  mount: string
  total: number
  used: number
  free: number
  filesystem: string
}

interface NetworkInfo {
  rx: number
  tx: number
  connections: number
  interfaces: Array<{
    name: string
    rx: number
    tx: number
    ip: string[]
  }>
}

interface ProcessInfo {
  pid: number
  name: string
  cpu: number
  memory: number
  status: string
}

export function useMonitoring() {
  const [metrics, setMetrics] = useState<Metric[]>([])
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [traces, setTraces] = useState<TraceSpan[]>([])
  const [alerts, setAlerts] = useState<Alert[]>([])
  const [activeTrace, setActiveTrace] = useState<TraceSpan | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [isProfiling, setIsProfiling] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // ==================== Profile Methods ====================

  const startProfile = useCallback(async (name: string) => {
    setIsProfiling(true)
    try {
      const id = await invoke('start_profiling', { name }) as string
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  const stopProfile = useCallback(async () => {
    setIsProfiling(false)
    try {
      const result = await invoke('stop_profiling') as any
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  // ==================== Metric Methods ====================

  const collectMetrics = useCallback(async () => {
    try {
      const newMetrics = await invoke('collect_metrics') as Metric[]
      setMetrics(prev => [...newMetrics, ...prev].slice(0, 1000))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  const getMetrics = useCallback(async (names: string[], timeRange?: { start: number; end: number }) => {
    setIsLoading(true)
    try {
      const result = await invoke('get_metrics', { names, timeRange }) as Metric[]
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [])

  // ==================== Log Methods ====================

  const getLogs = useCallback(async (filters?: {
    level?: string[]
    module?: string[]
    search?: string
    since?: number
    until?: number
    limit?: number
  }) => {
    setIsLoading(true)
    try {
      const results = await invoke('get_logs', { filters }) as LogEntry[]
      setLogs(results)
      return results
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [])

  // streamLogs uses Tauri event listening rather than passing a callback to invoke
  const streamLogs = useCallback(async (callback: (log: LogEntry) => void) => {
    try {
      // inform backend to begin streaming (command may be no-op)
      await invoke('stream_logs')

      // listen for incoming log events; event name chosen by backend
      const unsubscribe = await listen('log-entry', (event: any) => {
        const log = event.payload as LogEntry
        setLogs(prev => [log, ...prev].slice(0, 1000))
        callback(log)
      })

      return unsubscribe
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  const exportLogs = useCallback(async (format: 'json' | 'txt' | 'csv') => {
    try {
      const data = await invoke('export_logs', { format }) as string
      return data
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  // ==================== Trace Methods ====================

  const startTrace = useCallback(async (name: string, tags?: Record<string, string>) => {
    try {
      const id = await invoke('start_trace', { name, tags }) as string
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  const endTrace = useCallback(async (id: string) => {
    try {
      const trace = await invoke('end_trace', { id }) as TraceSpan
      setTraces(prev => [trace, ...prev].slice(0, 100))
      return trace
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  const getTrace = useCallback(async (id: string) => {
    try {
      const trace = await invoke('get_trace', { id }) as TraceSpan
      setActiveTrace(trace)
      return trace
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  const getTraces = useCallback(async (name?: string, timeRange?: { start: number; end: number }) => {
    try {
      const results = await invoke('get_traces', { name, timeRange }) as TraceSpan[]
      return results
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    }
  }, [])

  // ==================== Alert Methods ====================

  const getAlerts = useCallback(async (status?: 'active' | 'resolved' | 'acknowledged') => {
    try {
      const results = await invoke('get_alerts', { status }) as Alert[]
      setAlerts(results)
      return results
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    }
  }, [])

  const acknowledgeAlert = useCallback(async (id: string) => {
    try {
      await invoke('acknowledge_alert', { id })
      await getAlerts()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [getAlerts])

  const resolveAlert = useCallback(async (id: string) => {
    try {
      await invoke('resolve_alert', { id })
      await getAlerts()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [getAlerts])

  // ==================== System Info Methods ====================

  const getSystemInfo = useCallback(async () => {
    try {
      const info = await invoke('get_system_info') as SystemInfo
      return info
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  const getCPUInfo = useCallback(async () => {
    try {
      const info = await invoke('get_cpu_info') as CPUInfo
      return info
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  const getMemoryInfo = useCallback(async () => {
    try {
      const info = await invoke('get_memory_info') as MemoryInfo
      return info
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  const getDiskInfo = useCallback(async () => {
    try {
      const info = await invoke('get_disk_info') as DiskInfo[]
      return info
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    }
  }, [])

  const getNetworkInfo = useCallback(async () => {
    try {
      const info = await invoke('get_network_info') as NetworkInfo
      return info
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  const getProcessInfo = useCallback(async () => {
    try {
      const info = await invoke('get_process_info') as ProcessInfo[]
      return info
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    }
  }, [])

  // ==================== Utility Methods ====================

  const clearMetrics = useCallback(() => {
    setMetrics([])
  }, [])

  const clearLogs = useCallback(() => {
    setLogs([])
  }, [])

  const clearTraces = useCallback(() => {
    setTraces([])
    setActiveTrace(null)
  }, [])

  const clearAlerts = useCallback(() => {
    setAlerts([])
  }, [])

  // Auto-collect metrics every 5 seconds
  useEffect(() => {
    collectMetrics()
    const interval = setInterval(collectMetrics, 5000)
    return () => clearInterval(interval)
  }, [collectMetrics])

  return {
    // State
    metrics,
    logs,
    traces,
    alerts,
    activeTrace,
    isProfiling,
    isLoading,
    error,

    // Profile Methods
    startProfile,
    stopProfile,

    // Metric Methods
    collectMetrics,
    getMetrics,
    clearMetrics,

    // Log Methods
    getLogs,
    streamLogs,
    exportLogs,
    clearLogs,

    // Trace Methods
    startTrace,
    endTrace,
    getTrace,
    getTraces,
    clearTraces,

    // Alert Methods
    getAlerts,
    acknowledgeAlert,
    resolveAlert,
    clearAlerts,

    // System Info Methods
    getSystemInfo,
    getCPUInfo,
    getMemoryInfo,
    getDiskInfo,
    getNetworkInfo,
    getProcessInfo,
  }
}