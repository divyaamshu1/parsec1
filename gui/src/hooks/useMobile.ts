import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

interface Device {
  id: string
  name: string
  platform: 'android' | 'ios'
  model: string
  osVersion: string
  apiLevel?: number
  isEmulator: boolean
  isConnected: boolean
}

interface Emulator {
  id: string
  name: string
  platform: 'android' | 'ios'
  deviceType: string
  systemImage: string
  apiLevel?: number
  running: boolean
  vncPort?: number
  adbPort?: number
}

interface BuildResult {
  success: boolean
  outputPath: string
  duration: number
  logs: string
  error?: string
  errors?: string
}

interface ScreenshotOptions {
  quality?: number
  format?: 'png' | 'jpg'
}

export function useMobile() {
  const [devices, setDevices] = useState<Device[]>([])
  const [emulators, setEmulators] = useState<Emulator[]>([])
  const [activeDevice, setActiveDevice] = useState<string | null>(null)
  const [logs, setLogs] = useState<string[]>([])
  const [buildProgress, setBuildProgress] = useState(0)
  const [isLoading, setIsLoading] = useState(false)
  const [isBuilding, setIsBuilding] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // ==================== Device Discovery ====================

  const refreshDevices = useCallback(async () => {
    setIsLoading(true)
    try {
      const [deviceList, emulatorList] = await Promise.all([
        invoke('list_mobile_devices') as Promise<Device[]>,
        invoke('list_emulators') as Promise<Emulator[]>
      ])
      setDevices(deviceList)
      setEmulators(emulatorList)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // ==================== Device Selection ====================

  const setActiveDeviceCallback = useCallback((id: string | null) => {
    setActiveDevice(id)
    if (id) {
      getLogs(id)
    }
  }, [])

  // ==================== Emulator Control ====================

  const bootEmulator = useCallback(async (name: string) => {
    setIsLoading(true)
    try {
      const id = await invoke('start_emulator', { name }) as string
      await refreshDevices()
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refreshDevices])

  const shutdownEmulator = useCallback(async (id: string) => {
    setIsLoading(true)
    try {
      await invoke('stop_emulator', { id })
      await refreshDevices()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refreshDevices])

  const createEmulator = useCallback(async (config: {
    name: string
    deviceType: string
    systemImage: string
    apiLevel?: number
  }) => {
    setIsLoading(true)
    try {
      const id = await invoke('create_emulator', { config }) as string
      await refreshDevices()
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refreshDevices])

  // ==================== App Management ====================

  const installApp = useCallback(async (deviceId: string, appPath: string) => {
    setIsLoading(true)
    try {
      await invoke('install_app', { deviceId, appPath })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] App installed: ${appPath}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  const uninstallApp = useCallback(async (deviceId: string, packageName: string) => {
    setIsLoading(true)
    try {
      await invoke('uninstall_app', { deviceId, packageName })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] App uninstalled: ${packageName}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  const runApp = useCallback(async (deviceId: string, packageName: string, activity?: string) => {
    setIsLoading(true)
    try {
      await invoke('run_app', { deviceId, packageName, activity })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] App launched: ${packageName}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  const stopApp = useCallback(async (deviceId: string, packageName: string) => {
    setIsLoading(true)
    try {
      await invoke('stop_app', { deviceId, packageName })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] App stopped: ${packageName}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // ==================== Screenshot Methods ====================

  const takeScreenshot = useCallback(async (deviceId: string, options?: ScreenshotOptions): Promise<string> => {
    setIsLoading(true)
    try {
      const screenshot = await invoke('take_screenshot', { 
        deviceId, 
        quality: options?.quality || 90,
        format: options?.format || 'png'
      }) as string
      
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] Screenshot captured`])
      
      return `data:image/${options?.format || 'png'};base64,${screenshot}`
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // ==================== Build Methods ====================

  const buildAndroid = useCallback(async (projectPath: string, config?: {
    configuration?: 'debug' | 'release'
    clean?: boolean
    signingConfig?: {
      keystore: string
      password: string
      alias: string
    }
  }) => {
    setIsBuilding(true)
    setBuildProgress(10)
    try {
      const progressInterval = setInterval(() => {
        setBuildProgress(prev => Math.min(prev + 10, 90))
      }, 1000)

      const result = await invoke('build_android', { 
        projectPath, 
        config: {
          configuration: config?.configuration || 'debug',
          clean: config?.clean || false,
          signingConfig: config?.signingConfig
        }
      }) as BuildResult

      clearInterval(progressInterval)
      setBuildProgress(100)
      
      setLogs(prev => [...prev, 
        `[${new Date().toLocaleTimeString()}] Build ${result.success ? 'succeeded' : 'failed'}`,
        ...result.logs.split('\n').filter(l => l.trim())
      ])
      
      setTimeout(() => setBuildProgress(0), 2000)
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsBuilding(false)
    }
  }, [])

  const buildIOS = useCallback(async (projectPath: string, config?: {
    configuration?: 'debug' | 'release'
    clean?: boolean
    signingConfig?: {
      team: string
      certificate: string
      provisioning: string
    }
  }) => {
    setIsBuilding(true)
    setBuildProgress(10)
    try {
      const progressInterval = setInterval(() => {
        setBuildProgress(prev => Math.min(prev + 10, 90))
      }, 1000)

      const result = await invoke('build_ios', { 
        projectPath, 
        config: {
          configuration: config?.configuration || 'debug',
          clean: config?.clean || false,
          signingConfig: config?.signingConfig
        }
      }) as BuildResult

      clearInterval(progressInterval)
      setBuildProgress(100)
      
      setLogs(prev => [...prev, 
        `[${new Date().toLocaleTimeString()}] Build ${result.success ? 'succeeded' : 'failed'}`,
        ...result.logs.split('\n').filter(l => l.trim())
      ])
      
      setTimeout(() => setBuildProgress(0), 2000)
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsBuilding(false)
    }
  }, [])

  // ==================== Log Methods ====================

  const getLogs = useCallback(async (deviceId: string, filter?: string) => {
    setIsLoading(true)
    try {
      const logLines = await invoke('get_device_logs', { deviceId, filter }) as string[]
      setLogs(logLines)
      return logLines
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [])

  const clearLogs = useCallback(async (deviceId: string) => {
    try {
      await invoke('clear_device_logs', { deviceId })
      setLogs([])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  // ==================== Device Control ====================

  const sendKeyEvent = useCallback(async (deviceId: string, keyCode: string) => {
    try {
      await invoke('send_key_event', { deviceId, keyCode })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] Key event: ${keyCode}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  const sendTouchEvent = useCallback(async (deviceId: string, x: number, y: number, action: 'down' | 'up' | 'move') => {
    try {
      await invoke('send_touch_event', { deviceId, x, y, action })
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  const sendText = useCallback(async (deviceId: string, text: string) => {
    try {
      await invoke('send_text', { deviceId, text })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] Text sent: ${text}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  // ==================== File Operations ====================

  const pullFile = useCallback(async (deviceId: string, remotePath: string, localPath: string) => {
    setIsLoading(true)
    try {
      await invoke('pull_file', { deviceId, remotePath, localPath })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] File pulled: ${remotePath}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  const pushFile = useCallback(async (deviceId: string, localPath: string, remotePath: string) => {
    setIsLoading(true)
    try {
      await invoke('push_file', { deviceId, localPath, remotePath })
      setLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] File pushed: ${localPath}`])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // ==================== Utility Methods ====================

  const getDevice = useCallback((id: string) => {
    return devices.find(d => d.id === id) || emulators.find(e => e.id === id)
  }, [devices, emulators])

  const getEmulatorById = useCallback((id: string) => {
    return emulators.find(e => e.id === id)
  }, [emulators])

  const isDeviceConnected = useCallback((id: string) => {
    const device = devices.find(d => d.id === id)
    return device?.isConnected || false
  }, [devices])

  // Load initial devices
  useEffect(() => {
    refreshDevices()
    
    // Refresh every 5 seconds
    const interval = setInterval(refreshDevices, 5000)
    return () => clearInterval(interval)
  }, [refreshDevices])

  return {
    // State
    devices,
    emulators,
    activeDevice,
    logs,
    isLoading,
    isBuilding,
    error,
    buildProgress,

    // Device Management - MATCHING THE COMPONENT EXPECTATIONS
    refreshDevices,
    setActiveDevice: setActiveDeviceCallback,
    getDevice,
    getEmulatorById,
    isDeviceConnected,

    // Emulator Control - MATCHING THE COMPONENT EXPECTATIONS
    bootEmulator,
    shutdownEmulator,
    createEmulator,

    // App Management - MATCHING THE COMPONENT EXPECTATIONS
    installApp,      // The component calls this 'install'
    uninstallApp,    // The component calls this 'uninstall'
    runApp,          // The component calls this 'launch'
    stopApp,

    // Screenshot Methods
    takeScreenshot,

    // Build Methods
    buildAndroid,
    buildIOS,

    // Log Methods - MATCHING THE COMPONENT EXPECTATIONS
    getLogs,         // The component calls this directly
    clearLogs,

    // Device Control
    sendKeyEvent,
    sendTouchEvent,
    sendText,

    // File Operations
    pullFile,
    pushFile,
  }
}