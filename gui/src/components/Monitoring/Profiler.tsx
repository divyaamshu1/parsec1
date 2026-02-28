import { useState, useEffect } from 'react'
import { useMonitoring } from '../../hooks/useMonitoring'
import { Play, Square, Download, RefreshCw, Cpu, Zap, HardDrive, Globe } from 'lucide-react'

export default function Profiler() {
  const { 
    startProfile,
    stopProfile,
    getCPUInfo,
    getMemoryInfo,
    getProcessInfo,
    isLoading
  } = useMonitoring()

  const [profiling, setProfiling] = useState(false)
  const [cpuData, setCpuData] = useState<any[]>([])
  const [memoryData, setMemoryData] = useState<any[]>([])
  const [processes, setProcesses] = useState<any[]>([])
  const [selectedTab, setSelectedTab] = useState<'cpu' | 'memory' | 'processes'>('cpu')
  const [profileName, setProfileName] = useState('')

  useEffect(() => {
    if (profiling) {
      const interval = setInterval(collectData, 1000)
      return () => clearInterval(interval)
    }
  }, [profiling])

  const collectData = async () => {
    const cpu = await getCPUInfo()
    const memory = await getMemoryInfo()
    const proc = await getProcessInfo()

    setCpuData(prev => [...prev, { timestamp: Date.now(), ...cpu }].slice(-60))
    setMemoryData(prev => [...prev, { timestamp: Date.now(), ...memory }].slice(-60))
    setProcesses(proc)
  }

  const handleStartProfiling = async () => {
    await startProfile(profileName || `Profile-${Date.now()}`)
    setProfiling(true)
  }

  const handleStopProfiling = async () => {
    const result = await stopProfile()
    setProfiling(false)
    
    // Download profile data
    const blob = new Blob([JSON.stringify(result, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${profileName || 'profile'}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const renderCpuChart = () => {
    const maxCpu = Math.max(...cpuData.map(d => d.usage || 0), 100)
    
    return (
      <div className="chart-container">
        <div className="chart">
          {cpuData.map((point, i) => (
            <div
              key={i}
              className="chart-bar"
              style={{
                height: `${(point.usage / maxCpu) * 100}%`,
                left: `${(i / cpuData.length) * 100}%`
              }}
            >
              <span className="bar-value">{point.usage}%</span>
            </div>
          ))}
        </div>
        <div className="chart-labels">
          <span>Now</span>
          <span>-60s</span>
        </div>
      </div>
    )
  }

  const renderMemoryChart = () => {
    const maxMem = Math.max(...memoryData.map(d => d.used || 0), 1)
    
    return (
      <div className="chart-container">
        <div className="chart">
          {memoryData.map((point, i) => (
            <div
              key={i}
              className="chart-bar memory"
              style={{
                height: `${(point.used / point.total) * 100}%`,
                left: `${(i / memoryData.length) * 100}%`
              }}
            >
              <span className="bar-value">
                {Math.round(point.used / 1024 / 1024)}MB
              </span>
            </div>
          ))}
        </div>
        <div className="chart-labels">
          <span>Now</span>
          <span>-60s</span>
        </div>
      </div>
    )
  }

  const renderProcesses = () => {
    const sorted = [...processes].sort((a, b) => b.cpu - a.cpu).slice(0, 20)

    return (
      <table className="process-table">
        <thead>
          <tr>
            <th>PID</th>
            <th>Name</th>
            <th>CPU %</th>
            <th>Memory</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map(proc => (
            <tr key={proc.pid}>
              <td>{proc.pid}</td>
              <td>{proc.name}</td>
              <td>{proc.cpu?.toFixed(1)}%</td>
              <td>{(proc.memory / 1024 / 1024).toFixed(1)} MB</td>
              <td>{proc.status}</td>
            </tr>
          ))}
        </tbody>
      </table>
    )
  }

  return (
    <div className="profiler">
      <div className="profiler-header">
        <h2>System Profiler</h2>
        <div className="profiler-controls">
          <input
            type="text"
            placeholder="Profile name"
            value={profileName}
            onChange={(e) => setProfileName(e.target.value)}
            disabled={profiling}
          />
          {profiling ? (
            <button onClick={handleStopProfiling} className="danger">
              <Square size={16} /> Stop Profiling
            </button>
          ) : (
            <button onClick={handleStartProfiling}>
              <Play size={16} /> Start Profiling
            </button>
          )}
          <button onClick={collectData} disabled={isLoading}>
            <RefreshCw size={16} className={isLoading ? 'spin' : ''} />
          </button>
        </div>
      </div>

      <div className="profiler-tabs">
        <button
          className={selectedTab === 'cpu' ? 'active' : ''}
          onClick={() => setSelectedTab('cpu')}
        >
          <Cpu size={16} /> CPU
        </button>
        <button
          className={selectedTab === 'memory' ? 'active' : ''}
          onClick={() => setSelectedTab('memory')}
        >
          <Zap size={16} /> Memory
        </button>
        <button
          className={selectedTab === 'processes' ? 'active' : ''}
          onClick={() => setSelectedTab('processes')}
        >
          <HardDrive size={16} /> Processes
        </button>
      </div>

      <div className="profiler-content">
        {selectedTab === 'cpu' && (
          <div className="cpu-view">
            <div className="current-stats">
              <div className="stat">
                <label>Current CPU</label>
                <span className="value">{cpuData[cpuData.length - 1]?.usage || 0}%</span>
              </div>
              <div className="stat">
                <label>Average</label>
                <span className="value">
                  {(cpuData.reduce((acc, d) => acc + (d.usage || 0), 0) / cpuData.length || 0).toFixed(1)}%
                </span>
              </div>
              <div className="stat">
                <label>Peak</label>
                <span className="value">{Math.max(...cpuData.map(d => d.usage || 0))}%</span>
              </div>
            </div>
            {renderCpuChart()}
          </div>
        )}

        {selectedTab === 'memory' && (
          <div className="memory-view">
            <div className="current-stats">
              <div className="stat">
                <label>Used Memory</label>
                <span className="value">
                  {Math.round((memoryData[memoryData.length - 1]?.used || 0) / 1024 / 1024)} MB
                </span>
              </div>
              <div className="stat">
                <label>Total Memory</label>
                <span className="value">
                  {Math.round((memoryData[memoryData.length - 1]?.total || 0) / 1024 / 1024)} MB
                </span>
              </div>
              <div className="stat">
                <label>Usage</label>
                <span className="value">
                  {((memoryData[memoryData.length - 1]?.used / memoryData[memoryData.length - 1]?.total) * 100 || 0).toFixed(1)}%
                </span>
              </div>
            </div>
            {renderMemoryChart()}
          </div>
        )}

        {selectedTab === 'processes' && renderProcesses()}
      </div>
    </div>
  )
}