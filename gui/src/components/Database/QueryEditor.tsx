import { useState, useEffect } from 'react'
import { useDatabase } from '../../hooks/useDatabase'
import Editor from '@monaco-editor/react'
import { Play, Download, Clock, Save } from 'lucide-react'

export default function QueryEditor() {
  const { 
    activeConnection,
    queryHistory,
    queryResult,
    isLoading,
    runQuery,
    exportQueryResult
  } = useDatabase()

  const [query, setQuery] = useState('-- Write your SQL query here\nSELECT * FROM users LIMIT 10;')
  const [historyIndex, setHistoryIndex] = useState(-1)

  useEffect(() => {
    if (historyIndex >= 0 && historyIndex < queryHistory.length) {
      setQuery(queryHistory[historyIndex].query)
    }
  }, [historyIndex, queryHistory])

  const handleRunQuery = async () => {
    if (!activeConnection || !query.trim()) return
    await runQuery(query)
  }

  const handleHistoryPrev = () => {
    if (historyIndex < queryHistory.length - 1) {
      setHistoryIndex(historyIndex + 1)
    }
  }

  const handleHistoryNext = () => {
    if (historyIndex > 0) {
      setHistoryIndex(historyIndex - 1)
    } else if (historyIndex === 0) {
      setHistoryIndex(-1)
      setQuery('')
    }
  }

  const handleExport = async (format: 'json' | 'csv' | 'sql') => {
    if (!queryResult) return
    const data = await exportQueryResult(format)
    
    // Download file
    const blob = new Blob([data], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `query-result.${format}`
    a.click()
    URL.revokeObjectURL(url)
  }

  if (!activeConnection) {
    return (
      <div className="query-editor empty">
        <div className="empty-state">
          <h3>No Database Connected</h3>
          <p>Connect to a database to run queries</p>
        </div>
      </div>
    )
  }

  return (
    <div className="query-editor">
      <div className="query-toolbar">
        <div className="toolbar-left">
          <button onClick={handleRunQuery} disabled={isLoading}>
            <Play size={16} /> Run
          </button>
          <button onClick={handleHistoryPrev} disabled={historyIndex >= queryHistory.length - 1}>
            ↑ Prev
          </button>
          <button onClick={handleHistoryNext} disabled={historyIndex <= -1}>
            ↓ Next
          </button>
          <span className="history-info">
            {historyIndex >= 0 ? `${historyIndex + 1}/${queryHistory.length}` : ''}
          </span>
        </div>
        <div className="toolbar-right">
          {queryResult && (
            <>
              <button onClick={() => handleExport('json')}>
                <Download size={16} /> JSON
              </button>
              <button onClick={() => handleExport('csv')}>
                <Download size={16} /> CSV
              </button>
              <button onClick={() => handleExport('sql')}>
                <Save size={16} /> SQL
              </button>
            </>
          )}
        </div>
      </div>

      <div className="query-input">
        <Editor
          height="200px"
          defaultLanguage="sql"
          value={query}
          onChange={(value) => setQuery(value || '')}
          theme="vs-dark"
          options={{
            fontSize: 13,
            minimap: { enabled: false },
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            wordWrap: 'on'
          }}
        />
      </div>

      <div className="query-results">
        {isLoading && (
          <div className="loading-state">
            <div className="spinner" />
            <p>Running query...</p>
          </div>
        )}

        {queryResult && (
          <>
            <div className="results-header">
              <span>
                <Clock size={14} /> {queryResult.duration}ms
              </span>
              <span>
                {queryResult.rows?.length || 0} rows returned
                {queryResult.affected !== undefined && ` • ${queryResult.affected} rows affected`}
              </span>
            </div>

            <div className="results-table">
              {queryResult.columns && queryResult.columns.length > 0 ? (
                <table>
                  <thead>
                    <tr>
                      {queryResult.columns.map((col: string) => (
                        <th key={col}>{col}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {queryResult.rows?.map((row: any[], i: number) => (
                      <tr key={i}>
                        {row.map((cell, j) => (
                          <td key={j}>{cell?.toString() || 'NULL'}</td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <div className="no-results">
                  Query executed successfully. No results to display.
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  )
}