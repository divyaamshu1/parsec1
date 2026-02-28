import { useState, useCallback, useEffect } from 'react'
import { useDatabaseStore } from '../store/databaseStore'

export function useDatabase() {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [queryResult, setQueryResult] = useState<any>(null)
  
  const { 
    connections,
    activeConnection,
    databases,
    tables,
    queryHistory,
    loadConnections,
    addConnection,
    removeConnection,
    connect,
    disconnect,
    setActiveConnection,
    getDatabases,
    getTables,
    getTableSchema,
    executeQuery,
    executeQueryOnConnection,
    exportResults,
    clearHistory
  } = useDatabaseStore()

  const refreshConnections = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      await loadConnections()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [loadConnections])

  const createConnection = useCallback(async (conn: any) => {
    setIsLoading(true)
    setError(null)
    try {
      await addConnection(conn)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [addConnection])

  const deleteConnection = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await removeConnection(id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [removeConnection])

  const connectToDatabase = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await connect(id)
      await setActiveConnection(id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [connect, setActiveConnection])

  const disconnectFromDatabase = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await disconnect(id)
      if (activeConnection === id) {
        await setActiveConnection(null)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [disconnect, setActiveConnection, activeConnection])

  const fetchDatabases = useCallback(async () => {
    if (!activeConnection) return []
    
    setIsLoading(true)
    setError(null)
    try {
      const dbs = await getDatabases()
      return dbs
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [activeConnection, getDatabases])

  const fetchTables = useCallback(async (database: string) => {
    if (!activeConnection) return []
    
    setIsLoading(true)
    setError(null)
    try {
      const tbls = await getTables(database)
      return tbls
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [activeConnection, getTables])

  const runQuery = useCallback(async (query: string) => {
    if (!activeConnection) throw new Error('No database connected')
    
    setIsLoading(true)
    setError(null)
    setQueryResult(null)
    try {
      const result = await executeQuery(query)
      setQueryResult(result)
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [activeConnection, executeQuery])

  const runQueryOnConnection = useCallback(async (id: string, query: string) => {
    setIsLoading(true)
    setError(null)
    setQueryResult(null)
    try {
      const result = await executeQueryOnConnection(id, query)
      setQueryResult(result)
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [executeQueryOnConnection])

  const exportQueryResult = useCallback(async (format: 'json' | 'csv' | 'sql') => {
    if (!queryResult) return ''
    
    try {
      return await exportResults(format, queryResult)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return ''
    }
  }, [queryResult, exportResults])

  const getConnectionById = useCallback((id: string) => {
    return connections.find(c => c.id === id)
  }, [connections])

  const isConnected = useCallback((id: string) => {
    const conn = connections.find(c => c.id === id)
    return conn?.connected || false
  }, [connections])

  useEffect(() => {
    refreshConnections()
  }, [])

  return {
    // State
    connections,
    activeConnection,
    databases,
    tables,
    queryHistory,
    queryResult,
    isLoading,
    error,
    
    // Actions
    refreshConnections,
    createConnection,
    deleteConnection,
    connectToDatabase,
    disconnectFromDatabase,
    setActiveConnection,
    fetchDatabases,
    fetchTables,
    getTableSchema,
    runQuery,
    runQueryOnConnection,
    exportQueryResult,
    getConnectionById,
    isConnected,
    clearHistory
  }
}