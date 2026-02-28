import { useState } from 'react'
import { useCloud } from '../../hooks/useCloud'
import { Server, Database, Globe, HardDrive, Play, Square, Trash2, RefreshCw } from 'lucide-react'

export default function GCPExplorer() {
  const { 
    services, 
    isLoading, 
    error,
    start,
    stop,
    delete: deleteService,
    refreshServices
  } = useCloud()

  const [selectedProject, setSelectedProject] = useState<string>('all')
  const [selectedType, setSelectedType] = useState<string>('all')

  const projects = ['all', ...new Set(services.map(s => s.region))]
  const types = ['all', 'compute', 'storage', 'database', 'serverless']

  const filteredServices = services.filter(s => 
    (selectedProject === 'all' || s.region === selectedProject) &&
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

  if (error) {
    return <div className="error-message">{error}</div>
  }

  return (
    <div className="cloud-explorer">
      <div className="cloud-header">
        <h2>Google Cloud Platform</h2>
        <button onClick={refreshServices} disabled={isLoading}>
          <RefreshCw size={16} className={isLoading ? 'spin' : ''} />
        </button>
      </div>

      <div className="cloud-filters">
        <select value={selectedProject} onChange={(e) => setSelectedProject(e.target.value)}>
          {projects.map(p => (
            <option key={p} value={p}>{p === 'all' ? 'All Projects' : p}</option>
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
      </div>
    </div>
  )
}