import { useState } from 'react'
import { 
  Activity, Cpu, Zap, HardDrive, Globe,
  AlertCircle, TrendingUp, Clock
} from 'lucide-react'

export default function MonitoringPanel() {
  const [metrics] = useState({
    cpu: 45,
    memory: 62,
    disk: 38,
    network: 24
  })

  const [alerts] = useState([
    { level: 'warning', message: 'High memory usage', time: '2m ago' },
    { level: 'info', message: 'Build completed', time: '5m ago' },
    { level: 'error', message: 'Connection failed', time: '10m ago' },
  ])

  const [logs] = useState([
    { level: 'info', message: 'Server started', time: '10:00:00' },
    { level: 'debug', message: 'Loading config', time: '10:00:01' },
    { level: 'warn', message: 'Slow query detected', time: '10:00:05' },
  ])

  return (
    <div className="monitoring-panel">
      <div className="metrics-grid">
        <div className="metric-card">
          <Cpu size={16} />
          <div className="metric-value">{metrics.cpu}%</div>
          <div className="metric-label">CPU</div>
        </div>
        <div className="metric-card">
          <Zap size={16} />
          <div className="metric-value">{metrics.memory}%</div>
          <div className="metric-label">Memory</div>
        </div>
        <div className="metric-card">
          <HardDrive size={16} />
          <div className="metric-value">{metrics.disk}%</div>
          <div className="metric-label">Disk</div>
        </div>
        <div className="metric-card">
          <Globe size={16} />
          <div className="metric-value">{metrics.network}%</div>
          <div className="metric-label">Network</div>
        </div>
      </div>

      <div className="alerts-section">
        <h4>
          <AlertCircle size={14} /> Active Alerts
        </h4>
        {alerts.map((alert, i) => (
          <div key={i} className={`alert-item ${alert.level}`}>
            <span className="alert-message">{alert.message}</span>
            <span className="alert-time">{alert.time}</span>
          </div>
        ))}
      </div>

      <div className="logs-section">
        <h4>
          <Activity size={14} /> Live Logs
        </h4>
        <div className="logs-list">
          {logs.map((log, i) => (
            <div key={i} className={`log-item ${log.level}`}>
              <span className="log-time">{log.time}</span>
              <span className="log-level">[{log.level}]</span>
              <span className="log-message">{log.message}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="quick-stats">
        <div className="stat-item">
          <TrendingUp size={14} />
          <span>Uptime: 99.9%</span>
        </div>
        <div className="stat-item">
          <Clock size={14} />
          <span>Response: 245ms</span>
        </div>
      </div>
    </div>
  )
}