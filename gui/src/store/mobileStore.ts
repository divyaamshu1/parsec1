import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'

interface Device {
  id: string
  name: string
  platform: 'android' | 'ios'
  model: string
  osVersion: string
  isEmulator: boolean
  isConnected: boolean
  apiLevel?: number
}

interface Emulator {
  id: string
  name: string
  platform: 'android' | 'ios'
  deviceType: string
  systemImage: string
  running: boolean
  vncPort?: number
}

interface BuildResult {
  success: boolean
  outputPath: string
  duration: number
  logs: string
  // backend may report an error message when success is false
  error?: string
}

interface MobileState {
  devices: Device[]
  emulators: Emulator[]
  activeDevice: string | null
  building: boolean
  logs: string[]
  error: string | null

  loadDevices: () => Promise<void>
  loadEmulators: () => Promise<void>
  setActiveDevice: (id: string | null) => void
  
  startEmulator: (name: string) => Promise<string>
  stopEmulator: (id: string) => Promise<void>
  createEmulator: (config: any) => Promise<string>
  
  installApp: (deviceId: string, appPath: string) => Promise<void>
  uninstallApp: (deviceId: string, packageName: string) => Promise<void>
  runApp: (deviceId: string, packageName: string, activity?: string) => Promise<void>
  
  buildAndroid: (projectPath: string, config?: any) => Promise<BuildResult>
  buildIOS: (projectPath: string, config?: any) => Promise<BuildResult>
  
  takeScreenshot: (deviceId: string) => Promise<string>
  recordScreen: (deviceId: string, duration: number) => Promise<string>
  
  getLogs: (deviceId: string, filter?: string) => Promise<string[]>
  clearLogs: (deviceId: string) => Promise<void>
  
  sendKeyEvent: (deviceId: string, key: string) => Promise<void>
  sendTouch: (deviceId: string, x: number, y: number) => Promise<void>
  sendText: (deviceId: string, text: string) => Promise<void>
}

export const useMobileStore = create<MobileState>((set, get) => ({
  devices: [],
  emulators: [],
  activeDevice: null,
  building: false,
  logs: [],
  error: null,

  loadDevices: async () => {
    try {
      const devices = await invoke('list_mobile_devices') as Device[]
      set({ devices })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  loadEmulators: async () => {
    try {
      const emulators = await invoke('list_emulators') as Emulator[]
      set({ emulators })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  setActiveDevice: (id) => {
    set({ activeDevice: id })
  },

  startEmulator: async (name: string) => {
    try {
      const id = await invoke('start_emulator', { name }) as string
      await get().loadEmulators()
      return id
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  stopEmulator: async (id: string) => {
    try {
      await invoke('stop_emulator', { id })
      await get().loadEmulators()
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  createEmulator: async (config: any) => {
    try {
      const id = await invoke('create_emulator', { config }) as string
      await get().loadEmulators()
      return id
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  installApp: async (deviceId: string, appPath: string) => {
    try {
      await invoke('install_app', { deviceId, appPath })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  uninstallApp: async (deviceId: string, packageName: string) => {
    try {
      await invoke('uninstall_app', { deviceId, packageName })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  runApp: async (deviceId: string, packageName: string, activity?: string) => {
    try {
      await invoke('run_app', { deviceId, packageName, activity })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  buildAndroid: async (projectPath: string, config?: any) => {
    set({ building: true, error: null })
    try {
      const result = await invoke('build_android', { projectPath, config }) as BuildResult
      set({ building: false })
      return result
    } catch (error) {
      set({ error: String(error), building: false })
      throw error
    }
  },

  buildIOS: async (projectPath: string, config?: any) => {
    set({ building: true, error: null })
    try {
      const result = await invoke('build_ios', { projectPath, config }) as BuildResult
      set({ building: false })
      return result
    } catch (error) {
      set({ error: String(error), building: false })
      throw error
    }
  },

  takeScreenshot: async (deviceId: string) => {
    try {
      return await invoke('take_screenshot', { deviceId }) as string
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  recordScreen: async (deviceId: string, duration: number) => {
    try {
      return await invoke('record_screen', { deviceId, duration }) as string
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  getLogs: async (deviceId: string, filter?: string) => {
    try {
      const logs = await invoke('get_device_logs', { deviceId, filter }) as string[]
      set({ logs })
      return logs
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  clearLogs: async (deviceId: string) => {
    try {
      await invoke('clear_device_logs', { deviceId })
      set({ logs: [] })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  sendKeyEvent: async (deviceId: string, key: string) => {
    try {
      await invoke('send_key_event', { deviceId, key })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  sendTouch: async (deviceId: string, x: number, y: number) => {
    try {
      await invoke('send_touch_event', { deviceId, x, y })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  sendText: async (deviceId: string, text: string) => {
    try {
      await invoke('send_text', { deviceId, text })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  }
}))