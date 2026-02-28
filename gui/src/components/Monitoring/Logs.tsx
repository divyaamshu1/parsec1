import { useState, useEffect, useRef } from 'react'
import { useMonitoring } from '../../hooks/useMonitoring'
import { 
  Search, Filter, Download, Trash2, Pause, Play,
  AlertCircle, AlertTriangle, Info, Bug, X
} from 'lucide-react'

export default function Logs() {
  const { 
    getLogs,
    streamLogs,
    exportLogs,
    isLoading
  } = useMonitoring()

  const [logs, setLogs] = useState<any[]>([])
  const [filteredLogs, setFilteredLogs] = useState<any[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [levelFilter, setLevelFilter] = useState<string[]>([])
  const [moduleFilter, setModuleFilter] = useState<string[]>([])
  const [paused, setPaused] = useState(false)
  const [autoScroll, setAutoScroll] = useState(true)
  const logContainerRef = useRef<HTMLDivElement>(null)
  const unsubscribeRef = useRef<(() => void) | null>(null)

  const levels = ['error', 'warn', 'info', 'debug', 'trace']
  const modules = ['system', 'editor', 'terminal', 'git', 'network']

  useEffect(() => {
    startStreaming()
    return () => {
      if (unsubscribeRef.current) {
        unsubscribeRef.current()
      }
    }
  }, [])

  useEffect(() => {
    filterLogs()
  }, [logs, searchQuery, levelFilter, moduleFilter])

  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight
    }
  }, [filteredLogs, autoScroll])

  const startStreaming = async () => {
    const unsubscribe = await streamLogs((log: any) => {
      if (!paused) {
        setLogs(prev => [...prev, log].slice(-1000))
      }
    })
    unsubscribeRef.current = unsubscribe
  }

  const filterLogs = () => {
    let filtered = logs

    if (searchQuery) {
      filtered = filtered.filter(log => 
        log.message.toLowerCase().includes(searchQuery.toLowerCase()) ||
        log.module?.toLowerCase().includes(searchQuery.toLowerCase())
      )
    }

    if (levelFilter.length > 0) {
      filtered = filtered.filter(log => levelFilter.includes(log.level))
    }

    if (moduleFilter.length > 0) {
      filtered = filtered.filter(log => 
        log.module && moduleFilter.some(m => log.module.includes(m))
      )
    }

    setFilteredLogs(filtered)
  }

  const toggleLevel = (level: string) => {
    setLevelFilter(prev =>
      prev.includes(level)
        ? prev.filter(l => l !== level)
        : [...prev, level]
    )
  }

  const toggleModule = (module: string) => {
    setModuleFilter(prev =>
      prev.includes(module)
        ? prev.filter(m => m !== module)
        : [...prev, module]
    )
  }

  const handleExport = async () => {
    const data = await exportLogs('json')
    
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `logs-${Date.now()}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleClear = () => {
    setLogs([])
  }

  const getLevelIcon = (level: string) => {
    switch (level) {
      case 'error': return <AlertCircle size={14} className="error" />
      case 'warn': return <AlertTriangle size={14} className="warn" />
      case 'info': return <Info size={14} className="info" />
      case 'debug': return <Bug size={14} className="debug" />
      default: return null
    }
  }

  return (
    <div className="logs-viewer">
      <div className="logs-header">
        <h2>System Logs</h2>
        <div className="header-controls">
          <button onClick={() => setPaused(!paused)}>
            {paused ? <Play size={16} /> : <Pause size={16} />}
          </button>
          <button onClick={handleExport}>
            <Download size={16} />
          </button>
          <button onClick={handleClear}>
            <Trash2 size={16} />
          </button>
        </div>
      </div>

      <div className="logs-toolbar">
        <div className="search-bar">
          <Search size={16} />
          <input
            type="text"
            placeholder="Search logs..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          {searchQuery && (
            <button onClick={() => setSearchQuery('')}>
              <X size={14} />
            </button>
          )}
        </div>

        <div className="filter-group">
          <Filter size={16} />
          <div className="filter-dropdown">
            <span>Level</span>
            <div className="filter-options">
              {levels.map(level => (
                <label key={level}>
                  <input
                    type="checkbox"
                    checked={levelFilter.includes(level)}
                    onChange={() => toggleLevel(level)}
                  />
                  {level}
                </label>
              ))}
            </div>
          </div>

          <div className="filter-dropdown">
            <span>Module</span>
            <div className="filter-options">
              {modules.map(module => (
                <label key={module}>
                  <input
                    type="checkbox"
                    checked={moduleFilter.includes(module)}
                    onChange={() => toggleModule(module)}
                  />
                  {module}
                </label>
              ))}
            </div>
          </div>
        </div>

        <label className="auto-scroll">
          <input
            type="checkbox"
            checked={autoScroll}
            onChange={(e) => setAutoScroll(e.target.checked)}
          />
          Auto-scroll
        </label>
      </div>

      <div className="logs-container" ref={logContainerRef}>
        {filteredLogs.map((log, index) => (
          <div key={index} className={`log-entry ${log.level}`}>
            <span className="log-time">
              {new Date(log.timestamp).toLocaleTimeString()}
            </span>
            <span className="log-level">
              {getLevelIcon(log.level)}
              {log.level}
            </span>
            {log.module && (
              <span className="log-module">[{log.module}]</span>
            )}
            <span className="log-message">{log.message}</span>
            {log.file && (
              <span className="log-location">
                {log.file}:{log.line}
              </span>
            )}
          </div>
        ))}

        {filteredLogs.length === 0 && (
          <div className="no-logs">
            <p>No logs to display</p>
          </div>
        )}
      </div>

      <div className="logs-footer">
        <span>Showing {filteredLogs.length} of {logs.length} logs</span>
        {paused && <span className="paused-indicator">⏸ Paused</span>}
      </div>
    </div>
  )
}