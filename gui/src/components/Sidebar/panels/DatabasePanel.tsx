import { useState } from 'react'
import { 
  Database, Table, Play, Save, Plus,
  RefreshCw, ChevronRight, ChevronDown
} from 'lucide-react'

export default function DatabasePanel() {
  const [expanded, setExpanded] = useState<Set<string>>(new Set(['connections']))
  const [connections] = useState([
    { id: '1', name: 'Local PostgreSQL', type: 'postgres', host: 'localhost', port: 5432, connected: true },
    { id: '2', name: 'Production MySQL', type: 'mysql', host: 'db.example.com', port: 3306, connected: false },
    { id: '3', name: 'MongoDB Atlas', type: 'mongodb', host: 'cluster.mongodb.net', port: 27017, connected: false },
  ])

  const [tables] = useState([
    { name: 'users', rows: 1000, size: '1.2 MB' },
    { name: 'posts', rows: 5000, size: '5.8 MB' },
    { name: 'comments', rows: 15000, size: '12.4 MB' },
  ])

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expanded)
    if (expanded.has(section)) {
      newExpanded.delete(section)
    } else {
      newExpanded.add(section)
    }
    setExpanded(newExpanded)
  }

  return (
    <div className="database-panel">
      <div className="panel-actions">
        <button className="primary">
          <Plus size={14} /> New Connection
        </button>
        <button>
          <RefreshCw size={14} />
        </button>
      </div>

      <div className="connections-section">
        <div
          className="section-header"
          onClick={() => toggleSection('connections')}
        >
          {expanded.has('connections') ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span>CONNECTIONS</span>
        </div>

        {expanded.has('connections') && (
          <div className="connections-list">
            {connections.map(conn => (
              <div key={conn.id} className="connection-item">
                <Database size={14} />
                <div className="connection-info">
                  <div className="connection-name">{conn.name}</div>
                  <div className="connection-details">
                    {conn.host}:{conn.port}
                  </div>
                </div>
                <div className={`connection-status ${conn.connected ? 'connected' : ''}`} />
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="tables-section">
        <div
          className="section-header"
          onClick={() => toggleSection('tables')}
        >
          {expanded.has('tables') ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span>TABLES</span>
        </div>

        {expanded.has('tables') && (
          <div className="tables-list">
            {tables.map(table => (
              <div key={table.name} className="table-item">
                <Table size={14} />
                <span className="table-name">{table.name}</span>
                <span className="table-rows">{table.rows.toLocaleString()} rows</span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="query-section">
        <textarea
          placeholder="SELECT * FROM users LIMIT 10;"
          rows={4}
        />
        <div className="query-actions">
          <button className="primary">
            <Play size={14} /> Run
          </button>
          <button>
            <Save size={14} /> Save
          </button>
        </div>
      </div>
    </div>
  )
}