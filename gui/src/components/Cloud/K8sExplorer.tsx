import { useState } from 'react'
import { useCloud } from '../../hooks/useCloud'
import { Box, Server, HardDrive, Play, Square, Trash2, RefreshCw, GitBranch } from 'lucide-react'

export default function K8sExplorer() {
  const { 
    services, 
    isLoading, 
    error,
    start,
    stop,
    delete: deleteService,
    refreshServices,
    listK8sClusters,
    connectK8s
  } = useCloud()

  const [showConnect, setShowConnect] = useState(false)
  const [kubeConfig, setKubeConfig] = useState('')
  const [clusters, setClusters] = useState<any[]>([])

  const k8sServices = services.filter(s => s.type === 'compute')

  const handleConnect = async () => {
    try {
      await connectK8s(kubeConfig)
      const clusterList = await listK8sClusters()
      setClusters(clusterList)
      setShowConnect(false)
    } catch (error) {
      console.error('Failed to connect to cluster:', error)
    }
  }

  const getResourceIcon = (kind: string) => {
    switch (kind) {
      case 'pod': return Box
      case 'service': return Server
      case 'deployment': return GitBranch
      case 'volume': return HardDrive
      default: return Box
    }
  }

  return (
    <div className="cloud-explorer">
      <div className="cloud-header">
        <h2>Kubernetes</h2>
        <div className="header-actions">
          <button onClick={() => setShowConnect(!showConnect)}>
            Connect Cluster
          </button>
          <button onClick={refreshServices} disabled={isLoading}>
            <RefreshCw size={16} className={isLoading ? 'spin' : ''} />
          </button>
        </div>
      </div>

      {showConnect && (
        <div className="connect-panel">
          <textarea
            value={kubeConfig}
            onChange={(e) => setKubeConfig(e.target.value)}
            placeholder="Paste kubeconfig here..."
            rows={10}
          />
          <div className="connect-actions">
            <button onClick={handleConnect}>Connect</button>
            <button onClick={() => setShowConnect(false)}>Cancel</button>
          </div>
        </div>
      )}

      <div className="clusters-list">
        {clusters.map(cluster => (
          <div key={cluster.name} className="cluster-item">
            <div className="cluster-header">
              <span className="cluster-name">{cluster.name}</span>
              <span className="cluster-status">{cluster.status}</span>
            </div>
            <div className="cluster-resources">
              {/* Resources would be shown here */}
            </div>
          </div>
        ))}
      </div>

      <div className="k8s-services">
        <h3>Services</h3>
        {k8sServices.map(service => {
          const Icon = getResourceIcon(service.type)
          return (
            <div key={service.id} className="cloud-service">
              <div className="service-icon">
                <Icon size={24} />
              </div>
              <div className="service-info">
                <div className="service-name">{service.name}</div>
                <div className="service-meta">
                  <span className="service-type">{service.type}</span>
                </div>
              </div>
              <div className="service-status">
                <span className="status-text">{service.status}</span>
              </div>
              <div className="service-actions">
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