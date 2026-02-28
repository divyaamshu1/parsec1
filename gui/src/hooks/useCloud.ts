import { useState, useCallback, useEffect } from 'react'
import { useCloudStore } from '../store/cloudStore'

export function useCloud() {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [deploymentProgress, setDeploymentProgress] = useState<number>(0)
  
  const { 
    providers,
    activeProvider,
    services,
    regions,
    setActiveProvider,
    loadServices,
    loadRegions,
    deployFunction,
    deployContainer,
    deployDatabase,
    startService,
    stopService,
    deleteService,
    getLogs,
    getMetrics,
    syncWithK8s,
    getK8sClusters
  } = useCloudStore()

  const switchProvider = useCallback(async (provider: 'aws' | 'gcp' | 'azure') => {
    setIsLoading(true)
    setError(null)
    try {
      await setActiveProvider(provider)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [setActiveProvider])

  const refreshServices = useCallback(async () => {
    if (!activeProvider) return
    
    setIsLoading(true)
    setError(null)
    try {
      await loadServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [activeProvider, loadServices])

  const refreshRegions = useCallback(async () => {
    if (!activeProvider) return
    
    setIsLoading(true)
    setError(null)
    try {
      await loadRegions()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [activeProvider, loadRegions])

  const deployServerless = useCallback(async (name: string, runtime: string, code: string) => {
    if (!activeProvider) throw new Error('No cloud provider selected')
    
    setIsLoading(true)
    setError(null)
    setDeploymentProgress(0)
    try {
      // Simulate progress
      const interval = setInterval(() => {
        setDeploymentProgress(prev => Math.min(prev + 10, 90))
      }, 500)
      
      await deployFunction(name, runtime, code)
      
      clearInterval(interval)
      setDeploymentProgress(100)
      setTimeout(() => setDeploymentProgress(0), 1000)
      
      await refreshServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setDeploymentProgress(0)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [activeProvider, deployFunction, refreshServices])

  const deployContainerService = useCallback(async (name: string, image: string, port: number) => {
    if (!activeProvider) throw new Error('No cloud provider selected')
    
    setIsLoading(true)
    setError(null)
    setDeploymentProgress(0)
    try {
      const interval = setInterval(() => {
        setDeploymentProgress(prev => Math.min(prev + 10, 90))
      }, 500)
      
      await deployContainer(name, image, port)
      
      clearInterval(interval)
      setDeploymentProgress(100)
      setTimeout(() => setDeploymentProgress(0), 1000)
      
      await refreshServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setDeploymentProgress(0)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [activeProvider, deployContainer, refreshServices])

  const deployDatabaseService = useCallback(async (name: string, engine: string, size: string) => {
    if (!activeProvider) throw new Error('No cloud provider selected')
    
    setIsLoading(true)
    setError(null)
    setDeploymentProgress(0)
    try {
      const interval = setInterval(() => {
        setDeploymentProgress(prev => Math.min(prev + 10, 90))
      }, 500)
      
      await deployDatabase(name, engine, size)
      
      clearInterval(interval)
      setDeploymentProgress(100)
      setTimeout(() => setDeploymentProgress(0), 1000)
      
      await refreshServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setDeploymentProgress(0)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [activeProvider, deployDatabase, refreshServices])

  const start = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await startService(id)
      await refreshServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [startService, refreshServices])

  const stop = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await stopService(id)
      await refreshServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [stopService, refreshServices])

  const deleteSvc = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await deleteService(id)
      await refreshServices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [deleteService, refreshServices])

  const fetchLogs = useCallback(async (id: string, tail: number = 100) => {
    setIsLoading(true)
    setError(null)
    try {
      return await getLogs(id, tail)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [getLogs])

  const fetchMetrics = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      return await getMetrics(id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    } finally {
      setIsLoading(false)
    }
  }, [getMetrics])

  const connectK8s = useCallback(async (config: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await syncWithK8s(config)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [syncWithK8s])

  const listK8sClusters = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      return await getK8sClusters()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [getK8sClusters])

  const getServiceById = useCallback((id: string) => {
    return services.find(s => s.id === id)
  }, [services])

  const getServicesByType = useCallback((type: string) => {
    return services.filter(s => s.type === type)
  }, [services])

  const getServicesByRegion = useCallback((region: string) => {
    return services.filter(s => s.region === region)
  }, [services])

  useEffect(() => {
    if (activeProvider) {
      refreshServices()
      refreshRegions()
    }
  }, [activeProvider])

  return {
    // State
    providers,
    activeProvider,
    services,
    regions,
    isLoading,
    error,
    deploymentProgress,
    
    // Actions
    switchProvider,
    refreshServices,
    refreshRegions,
    deployServerless,
    deployContainerService,
    deployDatabaseService,
    start,
    stop,
    delete: deleteSvc,
    getLogs: fetchLogs,
    getMetrics: fetchMetrics,
    connectK8s,
    listK8sClusters,
    getServiceById,
    getServicesByType,
    getServicesByRegion
  }
}