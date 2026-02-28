import { useState } from 'react'
import { useAppStore } from '../../store/appStore'
import { 
  X, User, Palette, Keyboard, Terminal, Cloud,
  Database, Smartphone, Users, Brain, Eye,
  Sliders, Bell, Shield, Download, Upload
} from 'lucide-react'

export default function Settings() {
  const { closeModal, theme, toggleTheme } = useAppStore()
  const [activeTab, setActiveTab] = useState('general')
  const [settings, setSettings] = useState({
    editor: {
      fontSize: 14,
      tabSize: 4,
      wordWrap: true,
      minimap: true,
      lineNumbers: true,
      renderWhitespace: false,
      autoSave: true,
      formatOnSave: true
    },
    terminal: {
      fontSize: 13,
      fontFamily: 'Cascadia Code, monospace',
      cursorBlink: true,
      scrollback: 10000,
      shell: 'bash'
    },
    appearance: {
      theme: theme,
      sidebarPosition: 'left',
      activityBarPosition: 'left',
      panelPosition: 'bottom'
    },
    ai: {
      defaultProvider: 'openai',
      enableCompletions: true,
      enableChat: true,
      enableCodeActions: true
    },
    git: {
      autoFetch: true,
      fetchInterval: 60,
      enableGitLens: true
    },
    extensions: {
      autoUpdate: true,
      allowUnsafe: false,
      marketplace: 'official'
    }
  })

  const tabs = [
    { id: 'general', label: 'General', icon: Sliders },
    { id: 'editor', label: 'Editor', icon: Keyboard },
    { id: 'terminal', label: 'Terminal', icon: Terminal },
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'ai', label: 'AI', icon: Brain },
    { id: 'git', label: 'Git', icon: Users },
    { id: 'extensions', label: 'Extensions', icon: Download },
    { id: 'accessibility', label: 'Accessibility', icon: Eye },
    { id: 'privacy', label: 'Privacy', icon: Shield },
  ]

  const handleChange = (section: string, key: string, value: any) => {
    setSettings(prev => ({
      ...prev,
      [section]: {
        ...prev[section as keyof typeof prev],
        [key]: value
      }
    }))
  }

  const handleSave = () => {
    // Save settings to backend
    if (settings.appearance.theme !== theme) {
      toggleTheme()
    }
    closeModal()
  }

  const handleReset = () => {
    if (confirm('Reset all settings to default?')) {
      // Reset to defaults
      closeModal()
    }
  }

  return (
    <div className="modal-overlay" onClick={closeModal}>
      <div className="modal settings-modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Settings</h2>
          <button className="close-btn" onClick={closeModal}>
            <X size={18} />
          </button>
        </div>

        <div className="settings-container">
          <div className="settings-sidebar">
            {tabs.map(tab => {
              const Icon = tab.icon
              return (
                <button
                  key={tab.id}
                  className={`settings-tab ${activeTab === tab.id ? 'active' : ''}`}
                  onClick={() => setActiveTab(tab.id)}
                >
                  <Icon size={16} />
                  <span>{tab.label}</span>
                </button>
              )
            })}
          </div>

          <div className="settings-content">
            {activeTab === 'general' && (
              <div className="settings-section">
                <h3>General</h3>
                
                <div className="setting-item">
                  <label>Language</label>
                  <select>
                    <option>English</option>
                    <option>Spanish</option>
                    <option>French</option>
                    <option>German</option>
                    <option>Chinese</option>
                    <option>Japanese</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>Auto Save</label>
                  <select>
                    <option>off</option>
                    <option>afterDelay</option>
                    <option>onFocusChange</option>
                    <option>onWindowChange</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>Telemetry</label>
                  <select>
                    <option>all</option>
                    <option>error</option>
                    <option>off</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>
                    <input type="checkbox" defaultChecked />
                    Enable crash reports
                  </label>
                </div>
              </div>
            )}

            {activeTab === 'editor' && (
              <div className="settings-section">
                <h3>Editor</h3>

                <div className="setting-item">
                  <label>Font Size</label>
                  <input
                    type="number"
                    value={settings.editor.fontSize}
                    onChange={(e) => handleChange('editor', 'fontSize', parseInt(e.target.value))}
                    min="8"
                    max="32"
                  />
                </div>

                <div className="setting-item">
                  <label>Tab Size</label>
                  <input
                    type="number"
                    value={settings.editor.tabSize}
                    onChange={(e) => handleChange('editor', 'tabSize', parseInt(e.target.value))}
                    min="1"
                    max="8"
                  />
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.editor.wordWrap}
                      onChange={(e) => handleChange('editor', 'wordWrap', e.target.checked)}
                    />
                    Word Wrap
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.editor.minimap}
                      onChange={(e) => handleChange('editor', 'minimap', e.target.checked)}
                    />
                    Show Minimap
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.editor.lineNumbers}
                      onChange={(e) => handleChange('editor', 'lineNumbers', e.target.checked)}
                    />
                    Show Line Numbers
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.editor.formatOnSave}
                      onChange={(e) => handleChange('editor', 'formatOnSave', e.target.checked)}
                    />
                    Format on Save
                  </label>
                </div>
              </div>
            )}

            {activeTab === 'terminal' && (
              <div className="settings-section">
                <h3>Terminal</h3>

                <div className="setting-item">
                  <label>Font Size</label>
                  <input
                    type="number"
                    value={settings.terminal.fontSize}
                    onChange={(e) => handleChange('terminal', 'fontSize', parseInt(e.target.value))}
                    min="8"
                    max="32"
                  />
                </div>

                <div className="setting-item">
                  <label>Font Family</label>
                  <input
                    type="text"
                    value={settings.terminal.fontFamily}
                    onChange={(e) => handleChange('terminal', 'fontFamily', e.target.value)}
                  />
                </div>

                <div className="setting-item">
                  <label>Default Shell</label>
                  <select
                    value={settings.terminal.shell}
                    onChange={(e) => handleChange('terminal', 'shell', e.target.value)}
                  >
                    <option value="bash">bash</option>
                    <option value="zsh">zsh</option>
                    <option value="fish">fish</option>
                    <option value="powershell">PowerShell</option>
                    <option value="cmd">cmd</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>Scrollback Lines</label>
                  <input
                    type="number"
                    value={settings.terminal.scrollback}
                    onChange={(e) => handleChange('terminal', 'scrollback', parseInt(e.target.value))}
                    min="1000"
                    max="100000"
                    step="1000"
                  />
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.terminal.cursorBlink}
                      onChange={(e) => handleChange('terminal', 'cursorBlink', e.target.checked)}
                    />
                    Blinking Cursor
                  </label>
                </div>
              </div>
            )}

            {activeTab === 'appearance' && (
              <div className="settings-section">
                <h3>Appearance</h3>

                <div className="setting-item">
                  <label>Theme</label>
                  <select
                    value={settings.appearance.theme}
                    onChange={(e) => handleChange('appearance', 'theme', e.target.value)}
                  >
                    <option value="dark">Dark</option>
                    <option value="light">Light</option>
                    <option value="system">System</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>Sidebar Position</label>
                  <select
                    value={settings.appearance.sidebarPosition}
                    onChange={(e) => handleChange('appearance', 'sidebarPosition', e.target.value)}
                  >
                    <option value="left">Left</option>
                    <option value="right">Right</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>Panel Position</label>
                  <select
                    value={settings.appearance.panelPosition}
                    onChange={(e) => handleChange('appearance', 'panelPosition', e.target.value)}
                  >
                    <option value="bottom">Bottom</option>
                    <option value="right">Right</option>
                    <option value="left">Left</option>
                  </select>
                </div>
              </div>
            )}

            {activeTab === 'ai' && (
              <div className="settings-section">
                <h3>AI Settings</h3>

                <div className="setting-item">
                  <label>Default Provider</label>
                  <select
                    value={settings.ai.defaultProvider}
                    onChange={(e) => handleChange('ai', 'defaultProvider', e.target.value)}
                  >
                    <option value="openai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                    <option value="copilot">GitHub Copilot</option>
                    <option value="local">Local</option>
                  </select>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.ai.enableCompletions}
                      onChange={(e) => handleChange('ai', 'enableCompletions', e.target.checked)}
                    />
                    Enable Inline Completions
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.ai.enableChat}
                      onChange={(e) => handleChange('ai', 'enableChat', e.target.checked)}
                    />
                    Enable AI Chat
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.ai.enableCodeActions}
                      onChange={(e) => handleChange('ai', 'enableCodeActions', e.target.checked)}
                    />
                    Enable Code Actions
                  </label>
                </div>
              </div>
            )}

            {activeTab === 'git' && (
              <div className="settings-section">
                <h3>Git Settings</h3>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.git.autoFetch}
                      onChange={(e) => handleChange('git', 'autoFetch', e.target.checked)}
                    />
                    Auto Fetch
                  </label>
                </div>

                {settings.git.autoFetch && (
                  <div className="setting-item">
                    <label>Fetch Interval (seconds)</label>
                    <input
                      type="number"
                      value={settings.git.fetchInterval}
                      onChange={(e) => handleChange('git', 'fetchInterval', parseInt(e.target.value))}
                      min="10"
                      max="3600"
                    />
                  </div>
                )}

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.git.enableGitLens}
                      onChange={(e) => handleChange('git', 'enableGitLens', e.target.checked)}
                    />
                    Enable GitLens Features
                  </label>
                </div>
              </div>
            )}

            {activeTab === 'extensions' && (
              <div className="settings-section">
                <h3>Extensions</h3>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.extensions.autoUpdate}
                      onChange={(e) => handleChange('extensions', 'autoUpdate', e.target.checked)}
                    />
                    Auto Update Extensions
                  </label>
                </div>

                <div className="setting-item">
                  <label>Marketplace</label>
                  <select
                    value={settings.extensions.marketplace}
                    onChange={(e) => handleChange('extensions', 'marketplace', e.target.value)}
                  >
                    <option value="official">Official</option>
                    <option value="open">Open VSX</option>
                    <option value="custom">Custom</option>
                  </select>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.extensions.allowUnsafe}
                      onChange={(e) => handleChange('extensions', 'allowUnsafe', e.target.checked)}
                    />
                    Allow Unsafe Extensions
                  </label>
                </div>
              </div>
            )}

            {activeTab === 'accessibility' && (
              <div className="settings-section">
                <h3>Accessibility</h3>

                <div className="setting-item checkbox">
                  <label>
                    <input type="checkbox" />
                    Enable Screen Reader
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input type="checkbox" />
                    High Contrast Mode
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input type="checkbox" />
                    Reduce Motion
                  </label>
                </div>

                <div className="setting-item">
                  <label>Zoom Level</label>
                  <input type="range" min="0.5" max="2" step="0.1" defaultValue="1" />
                </div>
              </div>
            )}

            {activeTab === 'privacy' && (
              <div className="settings-section">
                <h3>Privacy</h3>

                <div className="setting-item checkbox">
                  <label>
                    <input type="checkbox" defaultChecked />
                    Enable Telemetry
                  </label>
                </div>

                <div className="setting-item checkbox">
                  <label>
                    <input type="checkbox" defaultChecked />
                    Send Crash Reports
                  </label>
                </div>

                <div className="setting-item">
                  <label>Privacy Policy</label>
                  <button>View Privacy Policy</button>
                </div>

                <div className="setting-item">
                  <button className="danger">Clear All Data</button>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="modal-footer">
          <button className="primary" onClick={handleSave}>Save Changes</button>
          <button onClick={handleReset}>Reset to Defaults</button>
          <button onClick={closeModal}>Cancel</button>
        </div>
      </div>
    </div>
  )
}