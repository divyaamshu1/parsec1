import { useState, useEffect } from 'react'
import { useMonitoring } from '../../hooks/useMonitoring'
import { 
  Activity, Cpu, HardDrive, Zap, Globe, Clock,
  AlertTriangle, CheckCircle, XCircle, RefreshCw
} from 'lucide-react'

export default function Dashboard() {
  const { 
    getSystemInfo,
    getCPUInfo,
    getMemoryInfo,
    getDiskInfo,
    getNetworkInfo,
    getAlerts,
    isLoading
  } = useMonitoring()

  const [systemInfo, setSystemInfo] = useState<any>(null)
  const [cpuInfo, setCpuInfo] = useState<any>(null)
  const [memoryInfo, setMemoryInfo] = useState<any>(null)
  const [diskInfo, setDiskInfo] = useState<any[]>([])
  const [networkInfo, setNetworkInfo] = useState<any>(null)
  const [alerts, setAlerts] = useState<any[]>([])
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date())

  useEffect(() => {
    loadData()
    const interval = setInterval(loadData, 5000)
    return () => clearInterval(interval)
  }, [])

  const loadData = async () => {
    const [
      sys,
      cpu,
      mem,
      disk,
      net,
      alertList
    ] = await Promise.all([
      getSystemInfo(),
      getCPUInfo(),
      getMemoryInfo(),
      getDiskInfo(),
      getNetworkInfo(),
      getAlerts()
    ])

    setSystemInfo(sys)
    setCpuInfo(cpu)
    setMemoryInfo(mem)
    setDiskInfo(disk)
    setNetworkInfo(net)
    setAlerts(alertList)
    setLastUpdated(new Date())
  }

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
  }

  const getUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    return `${days}d ${hours}h ${minutes}m`
  }

  const getAlertIcon = (severity: string) => {
    switch (severity) {
      case 'critical': return <XCircle size={16} className="critical" />
      case 'error': return <AlertTriangle size={16} className="error" />
      case 'warning': return <AlertTriangle size={16} className="warning" />
      default: return <CheckCircle size={16} className="info" />
    }
  }

  return (
    <div className="dashboard">
      <div className="dashboard-header">
        <h2>
          <Activity size={20} /> System Dashboard
        </h2>
        <div className="header-info">
          <Clock size={14} />
          <span>Last updated: {lastUpdated.toLocaleTimeString()}</span>
          <button onClick={loadData} disabled={isLoading}>
            <RefreshCw size={14} className={isLoading ? 'spin' : ''} />
          </button>
        </div>
      </div>

      <div className="dashboard-grid">
        <div className="dashboard-card system-info">
          <h3>
            <Zap size={16} /> System
          </h3>
          {systemInfo && (
            <div className="card-content">
              <div className="info-row">
                <span>Hostname</span>
                <span>{systemInfo.hostname}</span>
              </div>
              <div className="info-row">
                <span>OS</span>
                <span>{systemInfo.os} {systemInfo.arch}</span>
              </div>
              <div className="info-row">
                <span>Kernel</span>
                <span>{systemInfo.kernel}</span>
              </div>
              <div className="info-row">
                <span>Uptime</span>
                <span>{getUptime(systemInfo.uptime)}</span>
              </div>
              <div className="info-row">
                <span>Processes</span>
                <span>{systemInfo.processes}</span>
              </div>
            </div>
          )}
        </div>

        <div className="dashboard-card cpu-info">
          <h3>
            <Cpu size={16} /> CPU
          </h3>
          {cpuInfo && (
            <div className="card-content">
              <div className="metric-large">
                <span className="value">{cpuInfo.usage?.toFixed(1)}%</span>
                <span className="label">Usage</span>
              </div>
              <div className="info-row">
                <span>Cores</span>
                <span>{cpuInfo.cores}</span>
              </div>
              <div className="info-row">
                <span>Model</span>
                <span>{cpuInfo.model}</span>
              </div>
              <div className="info-row">
                <span>Load</span>
                <span>{cpuInfo.load?.toFixed(2)}</span>
              </div>
            </div>
          )}
        </div>

        <div className="dashboard-card memory-info">
          <h3>
            <Zap size={16} /> Memory
          </h3>
          {memoryInfo && (
            <div className="card-content">
              <div className="metric-large">
                <span className="value">
                  {((memoryInfo.used / memoryInfo.total) * 100).toFixed(1)}%
                </span>
                <span className="label">Used</span>
              </div>
              <div className="info-row">
                <span>Total</span>
                <span>{formatBytes(memoryInfo.total)}</span>
              </div>
              <div className="info-row">
                <span>Used</span>
                <span>{formatBytes(memoryInfo.used)}</span>
              </div>
              <div className="info-row">
                <span>Free</span>
                <span>{formatBytes(memoryInfo.free)}</span>
              </div>
            </div>
          )}
        </div>

        <div className="dashboard-card disk-info">
          <h3>
            <HardDrive size={16} /> Disk
          </h3>
          {diskInfo.length > 0 && (
            <div className="card-content">
              {diskInfo.slice(0, 3).map(disk => (
                <div key={disk.mount} className="disk-item">
                  <div className="disk-header">
                    <span>{disk.mount}</span>
                    <span>{disk.used} / {disk.total}</span>
                  </div>
                  <div className="progress-bar">
                    <div 
                      className="progress-fill"
                      style={{ width: `${(disk.used / disk.total) * 100}%` }}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="dashboard-card network-info">
          <h3>
            <Globe size={16} /> Network
          </h3>
          {networkInfo && (
            <div className="card-content">
              <div className="info-row">
                <span>RX</span>
                <span>{formatBytes(networkInfo.rx)}/s</span>
              </div>
              <div className="info-row">
                <span>TX</span>
                <span>{formatBytes(networkInfo.tx)}/s</span>
              </div>
              <div className="info-row">
                <span>Connections</span>
                <span>{networkInfo.connections}</span>
              </div>
            </div>
          )}
        </div>

        <div className="dashboard-card alerts-info">
          <h3>
            <AlertTriangle size={16} /> Active Alerts
          </h3>
          <div className="card-content">
            {alerts.filter(a => a.status === 'active').length === 0 ? (
              <div className="no-alerts">No active alerts</div>
            ) : (
              alerts
                .filter(a => a.status === 'active')
                .slice(0, 5)
                .map(alert => (
                  <div key={alert.id} className="alert-item">
                    {getAlertIcon(alert.severity)}
                    <span className="alert-message">{alert.message}</span>
                  </div>
                ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}