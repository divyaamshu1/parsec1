import { useState, useEffect } from 'react'
import { useDatabase } from '../../hooks/useDatabase'
import { Table, ChevronRight, ChevronDown, Eye } from 'lucide-react'

export default function TableViewer() {
  const { 
    activeConnection,
    databases,
    tables,
    fetchTables,
    getTableSchema
  } = useDatabase()

  const [expandedDbs, setExpandedDbs] = useState<Set<string>>(new Set())
  const [selectedTable, setSelectedTable] = useState<string | null>(null)
  const [tableSchema, setTableSchema] = useState<any>(null)

  useEffect(() => {
    if (selectedTable) {
      loadTableSchema(selectedTable)
    }
  }, [selectedTable])

  const toggleDb = (db: string) => {
    const newExpanded = new Set(expandedDbs)
    if (expandedDbs.has(db)) {
      newExpanded.delete(db)
    } else {
      newExpanded.add(db)
      fetchTables(db)
    }
    setExpandedDbs(newExpanded)
  }

  const loadTableSchema = async (table: string) => {
    const schema = await getTableSchema(table)
    setTableSchema(schema)
  }

  if (!activeConnection) {
    return (
      <div className="table-viewer empty">
        <div className="empty-state">
          <h3>No Database Connected</h3>
          <p>Connect to a database to view tables</p>
        </div>
      </div>
    )
  }

  return (
    <div className="table-viewer">
      <div className="schema-browser">
        <h3>Databases</h3>
        <div className="db-list">
          {databases.map(db => (
            <div key={db} className="db-item">
              <div className="db-header" onClick={() => toggleDb(db)}>
                {expandedDbs.has(db) ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                <span className="db-name">{db}</span>
              </div>
              {expandedDbs.has(db) && (
                <div className="tables-list">
                  {tables
                    .filter(t => t.name.includes(db))
                    .map(table => (
                      <div
                        key={table.name}
                        className={`table-item ${selectedTable === table.name ? 'selected' : ''}`}
                        onClick={() => setSelectedTable(table.name)}
                      >
                        <Table size={14} />
                        <span className="table-name">{table.name}</span>
                        {table.rowCount && (
                          <span className="table-rows">{table.rowCount.toLocaleString()} rows</span>
                        )}
                      </div>
                    ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="table-details">
        {selectedTable && tableSchema ? (
          <>
            <div className="details-header">
              <h3>{selectedTable}</h3>
              <button>
                <Eye size={16} /> Preview Data
              </button>
            </div>

            <div className="schema-details">
              <h4>Columns</h4>
              <table className="schema-table">
                <thead>
                  <tr>
                    <th>Column</th>
                    <th>Type</th>
                    <th>Nullable</th>
                    <th>Key</th>
                    <th>Default</th>
                  </tr>
                </thead>
                <tbody>
                  {tableSchema.columns?.map((col: any) => (
                    <tr key={col.name}>
                      <td>
                        <strong>{col.name}</strong>
                      </td>
                      <td>{col.data_type}</td>
                      <td>{col.nullable ? 'YES' : 'NO'}</td>
                      <td>
                        {col.is_primary_key ? 'PRI' : col.is_unique ? 'UNI' : ''}
                      </td>
                      <td>{col.default || 'NULL'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              {tableSchema.foreign_keys?.length > 0 && (
                <>
                  <h4>Foreign Keys</h4>
                  <table className="schema-table">
                    <thead>
                      <tr>
                        <th>Column</th>
                        <th>References</th>
                      </tr>
                    </thead>
                    <tbody>
                      {tableSchema.foreign_keys.map((fk: any) => (
                        <tr key={fk.column}>
                          <td>{fk.column}</td>
                          <td>{fk.foreign_table}.{fk.foreign_column}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </>
              )}
            </div>
          </>
        ) : (
          <div className="no-selection">
            <p>Select a table to view schema</p>
          </div>
        )}
      </div>
    </div>
  )
}