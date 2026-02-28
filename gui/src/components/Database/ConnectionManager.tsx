import { useState } from 'react'
import { useDatabase } from '../../hooks/useDatabase'
import { Plus, Database, Wifi, WifiOff, Trash2, Power } from 'lucide-react'

export default function ConnectionManager() {
  const { 
    connections, 
    activeConnection,
    isLoading,
    connectToDatabase,
    disconnectFromDatabase,
    deleteConnection,
    setActiveConnection
  } = useDatabase()

  const [showNew, setShowNew] = useState(false)
  const [newConn, setNewConn] = useState({
    name: '',
    type: 'postgres',
    host: 'localhost',
    port: 5432,
    database: '',
    username: '',
    password: ''
  })

  const handleConnect = async (id: string) => {
    await connectToDatabase(id)
  }

  const handleDisconnect = async (id: string) => {
    await disconnectFromDatabase(id)
  }

  const handleDelete = async (id: string) => {
    if (confirm('Are you sure you want to delete this connection?')) {
      await deleteConnection(id)
    }
  }

  const handleCreate = () => {
    // Would call createConnection
    setShowNew(false)
  }

  const getDatabaseIcon = (type: string) => {
    switch (type) {
      case 'postgres': return '🐘'
      case 'mysql': return '🐬'
      case 'mongodb': return '🍃'
      case 'sqlite': return '📁'
      case 'redis': return '📀'
      default: return '💾'
    }
  }

  return (
    <div className="connection-manager">
      <div className="manager-header">
        <h3>Database Connections</h3>
        <button onClick={() => setShowNew(true)}>
          <Plus size={16} /> New Connection
        </button>
      </div>

      {showNew && (
        <div className="new-connection-form">
          <h4>New Connection</h4>
          <input
            type="text"
            placeholder="Connection Name"
            value={newConn.name}
            onChange={(e) => setNewConn({ ...newConn, name: e.target.value })}
          />
          <select
            value={newConn.type}
            onChange={(e) => setNewConn({ ...newConn, type: e.target.value })}
          >
            <option value="postgres">PostgreSQL</option>
            <option value="mysql">MySQL</option>
            <option value="mongodb">MongoDB</option>
            <option value="sqlite">SQLite</option>
            <option value="redis">Redis</option>
          </select>
          <input
            type="text"
            placeholder="Host"
            value={newConn.host}
            onChange={(e) => setNewConn({ ...newConn, host: e.target.value })}
          />
          <input
            type="number"
            placeholder="Port"
            value={newConn.port}
            onChange={(e) => setNewConn({ ...newConn, port: parseInt(e.target.value) })}
          />
          <input
            type="text"
            placeholder="Database Name"
            value={newConn.database}
            onChange={(e) => setNewConn({ ...newConn, database: e.target.value })}
          />
          <input
            type="text"
            placeholder="Username"
            value={newConn.username}
            onChange={(e) => setNewConn({ ...newConn, username: e.target.value })}
          />
          <input
            type="password"
            placeholder="Password"
            value={newConn.password}
            onChange={(e) => setNewConn({ ...newConn, password: e.target.value })}
          />
          <div className="form-actions">
            <button onClick={handleCreate}>Create</button>
            <button onClick={() => setShowNew(false)}>Cancel</button>
          </div>
        </div>
      )}

      <div className="connections-list">
        {connections.map(conn => (
          <div 
            key={conn.id} 
            className={`connection-item ${activeConnection === conn.id ? 'active' : ''}`}
            onClick={() => setActiveConnection(conn.id)}
          >
            <div className="connection-icon">
              {getDatabaseIcon(conn.type)}
            </div>
            <div className="connection-info">
              <div className="connection-name">{conn.name}</div>
              <div className="connection-details">
                {conn.type} • {conn.host}:{conn.port}
              </div>
            </div>
            <div className="connection-status">
              {conn.connected ? (
                <Wifi size={16} className="connected" />
              ) : (
                <WifiOff size={16} className="disconnected" />
              )}
            </div>
            <div className="connection-actions">
              {conn.connected ? (
                <button onClick={(e) => { e.stopPropagation(); handleDisconnect(conn.id); }}>
                  <Power size={14} />
                </button>
              ) : (
                <button onClick={(e) => { e.stopPropagation(); handleConnect(conn.id); }}>
                  <Power size={14} />
                </button>
              )}
              <button onClick={(e) => { e.stopPropagation(); handleDelete(conn.id); }}>
                <Trash2 size={14} />
              </button>
            </div>
          </div>
        ))}

        {connections.length === 0 && (
          <div className="empty-state">
            <p>No database connections</p>
          </div>
        )}
      </div>
    </div>
  )
}