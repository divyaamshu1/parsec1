import { useState } from 'react'
import { useMobile } from '../../hooks/useMobile'
import { Smartphone, Tablet, Monitor, Play, Square, RotateCw, Download, Trash2 } from 'lucide-react'

export default function DeviceManager() {
  const { 
    devices, 
    emulators,
    activeDevice,
    setActiveDevice,
    bootEmulator,
    shutdownEmulator,
    install,
    uninstall,
    launch,
    takeScreenshot,
    getLogs
  } = useMobile()

  const [selectedDevice, setSelectedDevice] = useState<string | null>(null)
  const [appPath, setAppPath] = useState('')
  const [packageName, setPackageName] = useState('')

  const getDeviceIcon = (platform: string) => {
    switch (platform) {
      case 'android': return Smartphone
      case 'ios': return Tablet
      default: return Monitor
    }
  }

  const handleInstall = async () => {
    if (!selectedDevice || !appPath) return
    await install(selectedDevice, appPath)
  }

  const handleRun = async () => {
    if (!selectedDevice || !packageName) return
    await launch(selectedDevice, packageName)
  }

  const handleScreenshot = async () => {
    if (!selectedDevice) return
    const screenshot = await takeScreenshot(selectedDevice)
    
    // Download screenshot
    const link = document.createElement('a')
    link.download = `screenshot-${Date.now()}.png`
    link.href = `data:image/png;base64,${screenshot}`
    link.click()
  }

  const handleLogs = async () => {
    if (!selectedDevice) return
    const logs = await getLogs(selectedDevice)
    
    // Download logs
    const blob = new Blob([logs.join('\n')], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.download = `logs-${Date.now()}.txt`
    link.href = url
    link.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="device-manager">
      <div className="device-list">
        <h3>Physical Devices</h3>
        {devices.filter(d => !d.isEmulator).map(device => {
          const Icon = getDeviceIcon(device.platform)
          return (
            <div
              key={device.id}
              className={`device-item ${activeDevice === device.id ? 'active' : ''}`}
              onClick={() => {
                setSelectedDevice(device.id)
                setActiveDevice(device.id)
              }}
            >
              <Icon size={20} />
              <div className="device-info">
                <div className="device-name">{device.name}</div>
                <div className="device-details">
                  {device.model} • {device.osVersion}
                </div>
              </div>
              <div className={`device-status ${device.isConnected ? 'connected' : 'disconnected'}`} />
            </div>
          )
        })}

        <h3>Emulators</h3>
        {emulators.map(emulator => (
          <div
            key={emulator.id}
            className={`device-item ${activeDevice === emulator.id ? 'active' : ''}`}
            onClick={() => {
              setSelectedDevice(emulator.id)
              setActiveDevice(emulator.id)
            }}
          >
            <Monitor size={20} />
            <div className="device-info">
              <div className="device-name">{emulator.name}</div>
              <div className="device-details">
                {emulator.deviceType} • {emulator.systemImage}
              </div>
            </div>
            <div className="emulator-actions">
              {emulator.running ? (
                <button onClick={(e) => { e.stopPropagation(); shutdownEmulator(emulator.id); }}>
                  <Square size={14} />
                </button>
              ) : (
                <button onClick={(e) => { e.stopPropagation(); bootEmulator(emulator.name); }}>
                  <Play size={14} />
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="device-actions">
        <h3>Device Actions</h3>
        
        <div className="action-group">
          <h4>Install App</h4>
          <input
            type="text"
            placeholder="App Path (.apk/.ipa)"
            value={appPath}
            onChange={(e) => setAppPath(e.target.value)}
          />
          <button onClick={handleInstall} disabled={!selectedDevice || !appPath}>
            <Download size={16} /> Install
          </button>
        </div>

        <div className="action-group">
          <h4>Run App</h4>
          <input
            type="text"
            placeholder="Package Name"
            value={packageName}
            onChange={(e) => setPackageName(e.target.value)}
          />
          <button onClick={handleRun} disabled={!selectedDevice || !packageName}>
            <Play size={16} /> Launch
          </button>
        </div>

        <div className="action-group">
          <h4>Diagnostics</h4>
          <div className="button-group">
            <button onClick={handleScreenshot} disabled={!selectedDevice}>
              📸 Screenshot
            </button>
            <button onClick={handleLogs} disabled={!selectedDevice}>
              📋 Get Logs
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}