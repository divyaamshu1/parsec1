import { useState, useEffect, useRef } from 'react'
import { useMobile } from '../../hooks/useMobile'
import { Play, Square, RotateCw, Home, ArrowLeft, Volume2, VolumeX } from 'lucide-react'

export default function EmulatorView() {
  const { 
    activeDevice, 
    emulators,
    getEmulatorById,
    screenshot,
    keyEvent,
    touchEvent
  } = useMobile()

  const [emulator, setEmulator] = useState<any>(null)
  const [screenshotUrl, setScreenshotUrl] = useState<string | null>(null)
  const [muted, setMuted] = useState(false)
  const [rotated, setRotated] = useState(false)
  const refreshInterval = useRef<any>()

  useEffect(() => {
    if (activeDevice) {
      const emu = getEmulatorById(activeDevice)
      setEmulator(emu)
      
      if (emu?.running) {
        startScreenshotRefresh()
      }
    }

    return () => {
      if (refreshInterval.current) {
        clearInterval(refreshInterval.current)
      }
    }
  }, [activeDevice])

  const startScreenshotRefresh = () => {
    refreshInterval.current = setInterval(async () => {
      if (activeDevice) {
        const img = await screenshot(activeDevice)
        setScreenshotUrl(`data:image/png;base64,${img}`)
      }
    }, 1000)
  }

  const handleTouch = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!activeDevice || !emulator?.running) return

    const rect = e.currentTarget.getBoundingClientRect()
    const x = Math.floor((e.clientX - rect.left) / rect.width * (rotated ? 1920 : 1080))
    const y = Math.floor((e.clientY - rect.top) / rect.height * (rotated ? 1080 : 1920))
    
    touchEvent(activeDevice, x, y)
  }

  const handleKeyPress = async (key: string) => {
    if (!activeDevice || !emulator?.running) return
    await keyEvent(activeDevice, key)
  }

  if (!emulator) {
    return (
      <div className="emulator-view empty">
        <div className="empty-state">
          <h3>No Emulator Selected</h3>
          <p>Select an emulator to view its screen</p>
        </div>
      </div>
    )
  }

  return (
    <div className="emulator-view">
      <div className="emulator-header">
        <div className="emulator-info">
          <h3>{emulator.name}</h3>
          <span className={`status ${emulator.running ? 'running' : 'stopped'}`}>
            {emulator.running ? 'Running' : 'Stopped'}
          </span>
        </div>
        <div className="emulator-controls">
          <button onClick={() => setMuted(!muted)}>
            {muted ? <VolumeX size={16} /> : <Volume2 size={16} />}
          </button>
          <button onClick={() => setRotated(!rotated)}>
            <RotateCw size={16} />
          </button>
        </div>
      </div>

      <div 
        className={`emulator-screen ${rotated ? 'rotated' : ''}`}
        onClick={handleTouch}
      >
        {screenshotUrl ? (
          <img src={screenshotUrl} alt="Emulator Screen" />
        ) : (
          <div className="screen-placeholder">
            <Monitor size={48} />
            <p>Screen not available</p>
          </div>
        )}
      </div>

      <div className="emulator-buttons">
        <button onClick={() => handleKeyPress('KEYCODE_HOME')}>
          <Home size={20} />
          <span>Home</span>
        </button>
        <button onClick={() => handleKeyPress('KEYCODE_BACK')}>
          <ArrowLeft size={20} />
          <span>Back</span>
        </button>
        <button onClick={() => handleKeyPress('KEYCODE_APP_SWITCH')}>
          □
          <span>Recent</span>
        </button>
        <button onClick={() => handleKeyPress('KEYCODE_POWER')}>
          ⏻
          <span>Power</span>
        </button>
        <button onClick={() => handleKeyPress('KEYCODE_VOLUME_UP')}>
          +
          <span>Vol+</span>
        </button>
        <button onClick={() => handleKeyPress('KEYCODE_VOLUME_DOWN')}>
          -
          <span>Vol-</span>
        </button>
      </div>
    </div>
  )
}