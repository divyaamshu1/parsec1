import { useState } from 'react'
import { 
  Eye, EyeOff, Volume2, Mic, Contrast,
  Type, MousePointer, Zap
} from 'lucide-react'

export default function AccessibilityPanel() {
  const [settings, setSettings] = useState({
    screenReader: false,
    highContrast: false,
    reduceMotion: false,
    largeText: false,
    voiceControl: false,
    colorBlind: false
  })

  const toggleSetting = (key: keyof typeof settings) => {
    setSettings(prev => ({ ...prev, [key]: !prev[key] }))
  }

  return (
    <div className="accessibility-panel">
      <div className="settings-group">
        <h4>Vision</h4>
        
        <label className="setting-item">
          <Eye size={16} />
          <span>Screen Reader</span>
          <input
            type="checkbox"
            checked={settings.screenReader}
            onChange={() => toggleSetting('screenReader')}
          />
        </label>

        <label className="setting-item">
          <Contrast size={16} />
          <span>High Contrast</span>
          <input
            type="checkbox"
            checked={settings.highContrast}
            onChange={() => toggleSetting('highContrast')}
          />
        </label>

        <label className="setting-item">
          <EyeOff size={16} />
          <span>Reduce Motion</span>
          <input
            type="checkbox"
            checked={settings.reduceMotion}
            onChange={() => toggleSetting('reduceMotion')}
          />
        </label>

        <label className="setting-item">
          <Type size={16} />
          <span>Large Text</span>
          <input
            type="checkbox"
            checked={settings.largeText}
            onChange={() => toggleSetting('largeText')}
          />
        </label>
      </div>

      <div className="settings-group">
        <h4>Hearing</h4>

        <label className="setting-item">
          <Volume2 size={16} />
          <span>Closed Captions</span>
          <input type="checkbox" />
        </label>

        <label className="setting-item">
          <Zap size={16} />
          <span>Visual Alerts</span>
          <input type="checkbox" />
        </label>
      </div>

      <div className="settings-group">
        <h4>Mobility</h4>

        <label className="setting-item">
          <MousePointer size={16} />
          <span>Sticky Keys</span>
          <input type="checkbox" />
        </label>

        <label className="setting-item">
          <Mic size={16} />
          <span>Voice Control</span>
          <input
            type="checkbox"
            checked={settings.voiceControl}
            onChange={() => toggleSetting('voiceControl')}
          />
        </label>
      </div>

      <div className="preview-section">
        <h4>Preview</h4>
        <div className={`preview-box ${settings.highContrast ? 'high-contrast' : ''}`}>
          <p>Sample text with current settings</p>
          <button>Button</button>
        </div>
      </div>
    </div>
  )
}