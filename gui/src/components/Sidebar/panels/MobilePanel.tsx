import { useState } from 'react'
import { 
  Smartphone, Tablet, Play, Square, RefreshCw,
  Download, Camera, RotateCw, Home
} from 'lucide-react'

export default function MobilePanel() {
  const [devices] = useState([
    { id: '1', name: 'Pixel 6', platform: 'android', api: '33', online: true },
    { id: '2', name: 'iPhone 14', platform: 'ios', version: '16.4', online: false },
    { id: '3', name: 'Pixel 4 Emulator', platform: 'android', api: '30', online: true, emulator: true },
  ])

  const [selectedDevice, setSelectedDevice] = useState<string | null>('1')

  return (
    <div className="mobile-panel">
      <div className="device-list">
        <h4>Devices</h4>
        {devices.map(device => (
          <div
            key={device.id}
            className={`device-item ${selectedDevice === device.id ? 'selected' : ''}`}
            onClick={() => setSelectedDevice(device.id)}
          >
            {device.platform === 'android' ? <Smartphone size={16} /> : <Tablet size={16} />}
            <div className="device-info">
              <div className="device-name">{device.name}</div>
              <div className="device-details">
                {device.platform === 'android' ? `API ${device.api}` : `iOS ${device.version}`}
                {device.emulator && ' (Emulator)'}
              </div>
            </div>
            <div className={`device-status ${device.online ? 'online' : 'offline'}`} />
          </div>
        ))}
      </div>

      <div className="device-actions">
        <div className="action-group">
          <button title="Install App">
            <Download size={14} /> Install
          </button>
          <button title="Run App">
            <Play size={14} /> Run
          </button>
          <button title="Stop App">
            <Square size={14} /> Stop
          </button>
        </div>

        <div className="action-group">
          <button title="Screenshot">
            <Camera size={14} /> Screenshot
          </button>
          <button title="Rotate">
            <RotateCw size={14} /> Rotate
          </button>
          <button title="Home">
            <Home size={14} /> Home
          </button>
        </div>

        <div className="action-group">
          <button title="Refresh">
            <RefreshCw size={14} /> Refresh
          </button>
        </div>
      </div>

      <div className="device-logs">
        <h4>Logs</h4>
        <div className="logs-content">
          <div className="log-line">[2024-01-01 10:00:00] App started</div>
          <div className="log-line">[2024-01-01 10:00:01] Connected to device</div>
          <div className="log-line">[2024-01-01 10:00:02] Installing APK...</div>
        </div>
      </div>
    </div>
  )
}