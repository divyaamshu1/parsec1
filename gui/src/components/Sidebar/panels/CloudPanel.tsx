import { useState } from 'react'
import { 
  Cloud, Server, Database, Globe, HardDrive,
  Play, Square, RefreshCw, Plus, ChevronRight
} from 'lucide-react'

export default function CloudPanel() {
  const [activeProvider, setActiveProvider] = useState<'aws' | 'gcp' | 'azure'>('aws')
  const [services, setServices] = useState([
    { id: '1', name: 'web-server', type: 'EC2', status: 'running', region: 'us-east-1' },
    { id: '2', name: 'api-lambda', type: 'Lambda', status: 'running', region: 'us-east-1' },
    { id: '3', name: 'database', type: 'RDS', status: 'stopped', region: 'us-west-2' },
  ])

  const providers = [
    { id: 'aws', name: 'AWS', icon: Cloud },
    { id: 'gcp', name: 'GCP', icon: Globe },
    { id: 'azure', name: 'Azure', icon: Database },
  ]

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running': return '#6a9955'
      case 'stopped': return '#f48771'
      default: return '#888'
    }
  }

  return (
    <div className="cloud-panel">
      <div className="provider-tabs">
        {providers.map(provider => {
          const Icon = provider.icon
          return (
            <button
              key={provider.id}
              className={activeProvider === provider.id ? 'active' : ''}
              onClick={() => setActiveProvider(provider.id as any)}
            >
              <Icon size={14} />
              {provider.name}
            </button>
          )
        })}
      </div>

      <div className="region-selector">
        <select>
          <option>us-east-1</option>
          <option>us-west-2</option>
          <option>eu-west-1</option>
          <option>ap-southeast-1</option>
        </select>
        <button title="Refresh">
          <RefreshCw size={14} />
        </button>
      </div>

      <div className="services-list">
        <div className="list-header">
          <span>Services</span>
          <button>
            <Plus size={14} /> New
          </button>
        </div>

        {services.map(service => (
          <div key={service.id} className="service-item">
            <div className="service-icon">
              {service.type === 'EC2' && <Server size={16} />}
              {service.type === 'Lambda' && <Cloud size={16} />}
              {service.type === 'RDS' && <Database size={16} />}
            </div>
            <div className="service-info">
              <div className="service-name">{service.name}</div>
              <div className="service-type">{service.type}</div>
            </div>
            <div className="service-status">
              <span 
                className="status-dot"
                style={{ backgroundColor: getStatusColor(service.status) }}
              />
            </div>
            <div className="service-actions">
              {service.status === 'running' ? (
                <button title="Stop">
                  <Square size={12} />
                </button>
              ) : (
                <button title="Start">
                  <Play size={12} />
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="quick-actions">
        <h4>Quick Actions</h4>
        <button className="action-item">
          <span>Deploy Function</span>
          <ChevronRight size={14} />
        </button>
        <button className="action-item">
          <span>Create Bucket</span>
          <ChevronRight size={14} />
        </button>
        <button className="action-item">
          <span>View Logs</span>
          <ChevronRight size={14} />
        </button>
      </div>
    </div>
  )
}