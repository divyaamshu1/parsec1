import { useState, useEffect } from 'react'
import { useCloud } from '../../hooks/useCloud'
import { Server, Database, Globe, HardDrive, Play, Square, Trash2, RefreshCw } from 'lucide-react'

export default function AWSExplorer() {
  const { 
    services, 
    isLoading, 
    error,
    start,
    stop,
    delete: deleteService,
    refreshServices
  } = useCloud()

  const [selectedRegion, setSelectedRegion] = useState<string>('all')
  const [selectedType, setSelectedType] = useState<string>('all')

  const regions = ['all', ...new Set(services.map(s => s.region))]
  const types = ['all', 'compute', 'storage', 'database', 'serverless']

  const filteredServices = services.filter(s => 
    (selectedRegion === 'all' || s.region === selectedRegion) &&
    (selectedType === 'all' || s.type === selectedType)
  )

  const getIcon = (type: string) => {
    switch (type) {
      case 'compute': return Server
      case 'storage': return HardDrive
      case 'database': return Database
      case 'serverless': return Globe
      default: return Server
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running': return '#6a9955'
      case 'stopped': return '#f48771'
      case 'error': return '#f48771'
      case 'creating': return '#cca700'
      default: return '#888'
    }
  }

  if (error) {
    return <div className="error-message">{error}</div>
  }

  return (
    <div className="cloud-explorer">
      <div className="cloud-header">
        <h2>AWS Explorer</h2>
        <button onClick={refreshServices} disabled={isLoading}>
          <RefreshCw size={16} className={isLoading ? 'spin' : ''} />
        </button>
      </div>

      <div className="cloud-filters">
        <select value={selectedRegion} onChange={(e) => setSelectedRegion(e.target.value)}>
          {regions.map(r => (
            <option key={r} value={r}>{r === 'all' ? 'All Regions' : r}</option>
          ))}
        </select>

        <select value={selectedType} onChange={(e) => setSelectedType(e.target.value)}>
          {types.map(t => (
            <option key={t} value={t}>{t === 'all' ? 'All Types' : t}</option>
          ))}
        </select>
      </div>

      <div className="cloud-services">
        {filteredServices.map(service => {
          const Icon = getIcon(service.type)
          return (
            <div key={service.id} className="cloud-service">
              <div className="service-icon">
                <Icon size={24} />
              </div>
              <div className="service-info">
                <div className="service-name">{service.name}</div>
                <div className="service-meta">
                  <span className="service-type">{service.type}</span>
                  <span className="service-region">{service.region}</span>
                </div>
              </div>
              <div className="service-status">
                <span 
                  className="status-indicator" 
                  style={{ backgroundColor: getStatusColor(service.status) }}
                />
                <span className="status-text">{service.status}</span>
              </div>
              <div className="service-actions">
                {service.status === 'running' ? (
                  <button onClick={() => stop(service.id)} title="Stop">
                    <Square size={16} />
                  </button>
                ) : service.status === 'stopped' ? (
                  <button onClick={() => start(service.id)} title="Start">
                    <Play size={16} />
                  </button>
                ) : null}
                <button onClick={() => deleteService(service.id)} title="Delete">
                  <Trash2 size={16} />
                </button>
              </div>
            </div>
          )
        })}

        {filteredServices.length === 0 && !isLoading && (
          <div className="empty-state">
            <p>No services found</p>
          </div>
        )}

        {isLoading && (
          <div className="loading-state">
            <div className="spinner" />
            <p>Loading services...</p>
          </div>
        )}
      </div>
    </div>
  )
}