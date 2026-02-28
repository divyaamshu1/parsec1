import { useState, useEffect } from 'react'
import { useAccessibility } from '../../hooks/useAccessibility'
import { Volume2, VolumeX, Mic, Settings, Play, Square, SkipForward, SkipBack } from 'lucide-react'

export default function ScreenReader() {
  const { 
    screenReaderEnabled,
    screenReaderMode,
    speaking,
    currentSpeech,
    speechRate,
    speechPitch,
    voices,
    currentVoice,
    toggleScreenReader,
    setScreenReaderMode,
    speak,
    stopSpeaking,
    pauseSpeaking,
    resumeSpeaking,
    setSpeechRate,
    setSpeechPitch,
    setVoice
  } = useAccessibility()

  const [testText, setTestText] = useState('Hello, this is a screen reader test.')
  const [showSettings, setShowSettings] = useState(false)

  const handleSpeak = () => {
    speak(testText)
  }

  const handleStop = () => {
    stopSpeaking()
  }

  const handlePause = () => {
    pauseSpeaking()
  }

  const handleResume = () => {
    resumeSpeaking()
  }

  const modes = [
    { value: 'always', label: 'Always On' },
    { value: 'ondemand', label: 'On Demand' },
    { value: 'auto', label: 'Auto' },
    { value: 'focus', label: 'Focus Only' }
  ]

  return (
    <div className="accessibility-panel screen-reader">
      <div className="panel-header">
        <h3>
          <Volume2 size={18} /> Screen Reader
        </h3>
        <label className="toggle">
          <input
            type="checkbox"
            checked={screenReaderEnabled}
            onChange={toggleScreenReader}
          />
          <span className="toggle-slider"></span>
        </label>
      </div>

      {screenReaderEnabled && (
        <div className="panel-content">
          <div className="status-bar">
            <div className={`status-indicator ${speaking ? 'speaking' : ''}`}>
              {speaking ? '🔊 Speaking' : '🔇 Idle'}
            </div>
            {currentSpeech && (
              <div className="current-speech">
                Now: "{currentSpeech}"
              </div>
            )}
          </div>

          <div className="mode-selector">
            <label>Mode</label>
            <select 
              value={screenReaderMode} 
              onChange={(e) => setScreenReaderMode(e.target.value)}
            >
              {modes.map(mode => (
                <option key={mode.value} value={mode.value}>
                  {mode.label}
                </option>
              ))}
            </select>
          </div>

          <div className="test-section">
            <h4>Test Screen Reader</h4>
            <textarea
              value={testText}
              onChange={(e) => setTestText(e.target.value)}
              rows={3}
              placeholder="Enter text to speak..."
            />
            <div className="test-controls">
              <button onClick={handleSpeak} disabled={speaking}>
                <Play size={16} /> Speak
              </button>
              <button onClick={handlePause} disabled={!speaking}>
                Pause
              </button>
              <button onClick={handleResume} disabled={!speaking}>
                Resume
              </button>
              <button onClick={handleStop} disabled={!speaking}>
                <Square size={16} /> Stop
              </button>
            </div>
          </div>

          <div className="voice-settings">
            <div className="settings-header">
              <h4>Voice Settings</h4>
              <button onClick={() => setShowSettings(!showSettings)}>
                <Settings size={16} />
              </button>
            </div>

            {showSettings && (
              <div className="settings-detail">
                <div className="setting-item">
                  <label>Voice</label>
                  <select 
                    value={currentVoice || ''} 
                    onChange={(e) => setVoice(e.target.value)}
                  >
                    <option value="">Default</option>
                    {voices.map(voice => (
                      <option key={voice.id} value={voice.id}>
                        {voice.name} ({voice.language})
                      </option>
                    ))}
                  </select>
                </div>

                <div className="setting-item">
                  <label>Rate: {speechRate} WPM</label>
                  <input
                    type="range"
                    min="80"
                    max="400"
                    value={speechRate}
                    onChange={(e) => setSpeechRate(Number(e.target.value))}
                  />
                  <div className="range-values">
                    <span>Slow</span>
                    <span>Fast</span>
                  </div>
                </div>

                <div className="setting-item">
                  <label>Pitch: {speechPitch.toFixed(1)}</label>
                  <input
                    type="range"
                    min="0.5"
                    max="2.0"
                    step="0.1"
                    value={speechPitch}
                    onChange={(e) => setSpeechPitch(Number(e.target.value))}
                  />
                  <div className="range-values">
                    <span>Low</span>
                    <span>High</span>
                  </div>
                </div>
              </div>
            )}
          </div>

          <div className="shortcuts-info">
            <h4>Keyboard Shortcuts</h4>
            <div className="shortcut-list">
              <div className="shortcut-item">
                <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>S</kbd>
                <span>Toggle Screen Reader</span>
              </div>
              <div className="shortcut-item">
                <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd>
                <span>Speak Selected Text</span>
              </div>
              <div className="shortcut-item">
                <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>P</kbd>
                <span>Pause/Resume</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}