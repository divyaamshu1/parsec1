import { useState, useEffect } from 'react'
import { useMonitoring } from '../../hooks/useMonitoring'
import { Activity, TrendingUp, TrendingDown, Minus, RefreshCw, Download } from 'lucide-react'

export default function Metrics() {
  const { 
    getMetrics,
    isLoading 
  } = useMonitoring()

  const [metrics, setMetrics] = useState<any[]>([])
  const [selectedMetric, setSelectedMetric] = useState<string | null>(null)
  const [timeRange, setTimeRange] = useState<'1h' | '6h' | '24h' | '7d'>('1h')
  const [metricHistory, setMetricHistory] = useState<any[]>([])

  useEffect(() => {
    loadMetrics()
    const interval = setInterval(loadMetrics, 5000)
    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    if (selectedMetric) {
      loadMetricHistory(selectedMetric)
    }
  }, [selectedMetric, timeRange])

  const loadMetrics = async () => {
    const data = await getMetrics([])
    setMetrics(data)
  }

  const loadMetricHistory = async (name: string) => {
    const end = Date.now()
    const start = end - (
      timeRange === '1h' ? 3600000 :
      timeRange === '6h' ? 21600000 :
      timeRange === '24h' ? 86400000 : 604800000
    )

    const history = await getMetrics([name], { start, end })
    setMetricHistory(history)
  }

  const getTrendIcon = (value: number, previous: number) => {
    if (value > previous * 1.1) return <TrendingUp size={16} className="trend-up" />
    if (value < previous * 0.9) return <TrendingDown size={16} className="trend-down" />
    return <Minus size={16} className="trend-neutral" />
  }

  const formatValue = (metric: any) => {
    switch (metric.unit) {
      case 'bytes':
        return `${(metric.value / 1024 / 1024).toFixed(2)} MB`
      case 'percent':
        return `${metric.value.toFixed(1)}%`
      case 'count':
        return metric.value.toLocaleString()
      default:
        return metric.value.toString()
    }
  }

  return (
    <div className="metrics">
      <div className="metrics-header">
        <h2>
          <Activity size={20} /> System Metrics
        </h2>
        <div className="header-controls">
          <button onClick={loadMetrics} disabled={isLoading}>
            <RefreshCw size={16} className={isLoading ? 'spin' : ''} />
          </button>
          <button>
            <Download size={16} />
          </button>
        </div>
      </div>

      <div className="metrics-grid">
        {metrics.map(metric => (
          <div
            key={metric.name}
            className={`metric-card ${selectedMetric === metric.name ? 'selected' : ''}`}
            onClick={() => setSelectedMetric(metric.name)}
          >
            <div className="metric-header">
              <span className="metric-name">{metric.name}</span>
              {metricHistory.length > 1 && getTrendIcon(
                metric.value,
                metricHistory[metricHistory.length - 2]?.value || metric.value
              )}
            </div>
            <div className="metric-value">{formatValue(metric)}</div>
            <div className="metric-footer">
              <span className="metric-unit">{metric.unit}</span>
              <span className="metric-time">
                {new Date(metric.timestamp).toLocaleTimeString()}
              </span>
            </div>
          </div>
        ))}
      </div>

      {selectedMetric && (
        <div className="metric-detail">
          <div className="detail-header">
            <h3>{selectedMetric}</h3>
            <div className="time-range">
              <button
                className={timeRange === '1h' ? 'active' : ''}
                onClick={() => setTimeRange('1h')}
              >
                1h
              </button>
              <button
                className={timeRange === '6h' ? 'active' : ''}
                onClick={() => setTimeRange('6h')}
              >
                6h
              </button>
              <button
                className={timeRange === '24h' ? 'active' : ''}
                onClick={() => setTimeRange('24h')}
              >
                24h
              </button>
              <button
                className={timeRange === '7d' ? 'active' : ''}
                onClick={() => setTimeRange('7d')}
              >
                7d
              </button>
            </div>
          </div>

          <div className="detail-chart">
            {metricHistory.map((point, i) => {
              const height = (point.value / Math.max(...metricHistory.map(p => p.value))) * 100
              return (
                <div
                  key={i}
                  className="chart-point"
                  style={{
                    left: `${(i / metricHistory.length) * 100}%`,
                    bottom: `${height}%`
                  }}
                >
                  <span className="point-value">{point.value}</span>
                </div>
              )
            })}
          </div>

          <div className="detail-stats">
            <div className="stat">
              <label>Min</label>
              <span>{Math.min(...metricHistory.map(p => p.value)).toFixed(2)}</span>
            </div>
            <div className="stat">
              <label>Max</label>
              <span>{Math.max(...metricHistory.map(p => p.value)).toFixed(2)}</span>
            </div>
            <div className="stat">
              <label>Avg</label>
              <span>{(metricHistory.reduce((a, b) => a + b.value, 0) / metricHistory.length).toFixed(2)}</span>
            </div>
            <div className="stat">
              <label>Current</label>
              <span>{metricHistory[metricHistory.length - 1]?.value.toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}