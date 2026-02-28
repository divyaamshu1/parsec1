import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'
import { CloudService } from '../types'

interface CloudState {
  providers: Array<'aws' | 'gcp' | 'azure'>
  activeProvider: 'aws' | 'gcp' | 'azure' | null
  services: CloudService[]
  regions: string[]
  loading: boolean
  error: string | null

  setActiveProvider: (provider: 'aws' | 'gcp' | 'azure') => Promise<void>
  loadServices: () => Promise<void>
  loadRegions: () => Promise<void>
  
  deployFunction: (name: string, runtime: string, code: string) => Promise<void>
  deployContainer: (name: string, image: string, port: number) => Promise<void>
  deployDatabase: (name: string, engine: string, size: string) => Promise<void>
  
  startService: (id: string) => Promise<void>
  stopService: (id: string) => Promise<void>
  deleteService: (id: string) => Promise<void>
  
  getLogs: (id: string, tail: number) => Promise<string[]>
  getMetrics: (id: string) => Promise<any>
  
  syncWithK8s: (config: string) => Promise<void>
  getK8sClusters: () => Promise<any[]>
}

export const useCloudStore = create<CloudState>((set, get) => ({
  providers: ['aws', 'gcp', 'azure'],
  activeProvider: null,
  services: [],
  regions: [],
  loading: false,
  error: null,

  setActiveProvider: async (provider) => {
    set({ activeProvider: provider, loading: true })
    try {
      await invoke('set_cloud_provider', { provider })
      await get().loadServices()
      await get().loadRegions()
    } catch (error) {
      set({ error: String(error) })
    } finally {
      set({ loading: false })
    }
  },

  loadServices: async () => {
    const { activeProvider } = get()
    if (!activeProvider) return

    set({ loading: true })
    try {
      const services = await invoke('list_cloud_services', { provider: activeProvider }) as CloudService[]
      set({ services })
    } catch (error) {
      set({ error: String(error) })
    } finally {
      set({ loading: false })
    }
  },

  loadRegions: async () => {
    const { activeProvider } = get()
    if (!activeProvider) return

    try {
      const regions = await invoke('get_cloud_regions', { provider: activeProvider }) as string[]
      set({ regions })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  deployFunction: async (name: string, runtime: string, code: string) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    set({ loading: true })
    try {
      await invoke('deploy_cloud_function', {
        provider: activeProvider,
        name,
        runtime,
        code
      })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  deployContainer: async (name: string, image: string, port: number) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    set({ loading: true })
    try {
      await invoke('deploy_container', {
        provider: activeProvider,
        name,
        image,
        port
      })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  deployDatabase: async (name: string, engine: string, size: string) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    set({ loading: true })
    try {
      await invoke('deploy_database', {
        provider: activeProvider,
        name,
        engine,
        size
      })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  startService: async (id: string) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    try {
      await invoke('start_cloud_service', { provider: activeProvider, id })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  stopService: async (id: string) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    try {
      await invoke('stop_cloud_service', { provider: activeProvider, id })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  deleteService: async (id: string) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    try {
      await invoke('delete_cloud_service', { provider: activeProvider, id })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getLogs: async (id: string, tail: number) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    try {
      return await invoke('get_cloud_logs', { provider: activeProvider, id, tail }) as string[]
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getMetrics: async (id: string) => {
    const { activeProvider } = get()
    if (!activeProvider) throw new Error('No provider selected')

    try {
      return await invoke('get_cloud_metrics', { provider: activeProvider, id })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  syncWithK8s: async (config: string) => {
    try {
      await invoke('sync_k8s_cluster', { config })
      await get().loadServices()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getK8sClusters: async () => {
    try {
      return await invoke('get_k8s_clusters') as any[]
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  }
}))