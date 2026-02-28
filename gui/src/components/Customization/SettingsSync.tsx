import { useState, useEffect } from 'react'
import { useCustomization } from '../../hooks/useCustomization'
import { 
  Cloud, GitBranch, HardDrive, RefreshCw, Check, AlertCircle,
  Upload, Download, Settings, Globe, Lock, Unlock
} from 'lucide-react'

export default function SettingsSync() {
  const { 
    syncEnabled,
    syncStatus,
    syncProviders,
    activeSyncProvider,
    lastSyncTime,
    syncConfig,
    enableSync,
    disableSync,
    setSyncProvider,
    syncNow,
    updateSyncConfig,
    testConnection,
    getSyncHistory
  } = useCustomization()

  const [selectedProvider, setSelectedProvider] = useState(activeSyncProvider || 'local')
  const [configForm, setConfigForm] = useState({
    endpoint: '',
    username: '',
    password: '',
    token: '',
    repository: '',
    branch: 'main',
    path: '',
    encrypt: false,
    autoSync: true,
    interval: 3600
  })
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null)
  const [showConfig, setShowConfig] = useState(false)
  const [history, setHistory] = useState<any[]>([])

  useEffect(() => {
    if (syncConfig) {
      setConfigForm({
        endpoint: syncConfig.endpoint || '',
        username: syncConfig.username || '',
        password: syncConfig.password || '',
        token: syncConfig.token || '',
        repository: syncConfig.repository || '',
        branch: syncConfig.branch || 'main',
        path: syncConfig.path || '',
        encrypt: syncConfig.encrypt || false,
        autoSync: syncConfig.autoSync || true,
        interval: syncConfig.interval || 3600
      })
    }
  }, [syncConfig])

  useEffect(() => {
    if (syncEnabled) {
      loadHistory()
    }
  }, [syncEnabled, lastSyncTime])

  const loadHistory = async () => {
    const hist = await getSyncHistory()
    setHistory(hist)
  }

  const handleProviderChange = async (provider: string) => {
    setSelectedProvider(provider)
    await setSyncProvider(provider)
  }

  const handleTestConnection = async () => {
    setTesting(true)
    const result = await testConnection(selectedProvider, configForm)
    setTestResult(result)
    setTesting(false)
  }

  const handleSaveConfig = async () => {
    await updateSyncConfig(configForm)
    setShowConfig(false)
  }

  const handleSyncNow = async () => {
    await syncNow()
  }

  const getProviderIcon = (provider: string) => {
    switch (provider) {
      case 'local': return HardDrive
      case 'git': return GitBranch
      case 'cloud': return Cloud
      default: return Globe
    }
  }

  const formatTime = (timestamp: number | null) => {
    if (!timestamp) return 'Never'
    const date = new Date(timestamp)
    return date.toLocaleString()
  }

  const getStatusColor = () => {
    switch (syncStatus) {
      case 'success': return '#6a9955'
      case 'syncing': return '#007acc'
      case 'error': return '#f48771'
      default: return '#888'
    }
  }

  return (
    <div className="customization-panel settings-sync">
      <div className="panel-header">
        <h3>
          <Cloud size={18} /> Settings Sync
        </h3>
        <label className="toggle">
          <input
            type="checkbox"
            checked={syncEnabled}
            onChange={(e) => e.target.checked ? enableSync() : disableSync()}
          />
          <span className="toggle-slider"></span>
        </label>
      </div>

      {syncEnabled && (
        <div className="panel-content">
          <div className="sync-status">
            <div className="status-indicator" style={{ color: getStatusColor() }}>
              {syncStatus === 'syncing' && <RefreshCw size={16} className="spin" />}
              {syncStatus === 'success' && <Check size={16} />}
              {syncStatus === 'error' && <AlertCircle size={16} />}
              <span>
                {syncStatus === 'syncing' && 'Syncing...'}
                {syncStatus === 'success' && 'Sync successful'}
                {syncStatus === 'error' && 'Sync failed'}
                {syncStatus === 'idle' && 'Idle'}
              </span>
            </div>
            <div className="last-sync">
              Last sync: {formatTime(lastSyncTime)}
            </div>
          </div>

          <div className="sync-actions">
            <button onClick={handleSyncNow} disabled={syncStatus === 'syncing'}>
              <RefreshCw size={16} className={syncStatus === 'syncing' ? 'spin' : ''} />
              Sync Now
            </button>
            <button onClick={() => setShowConfig(!showConfig)}>
              <Settings size={16} /> Configure
            </button>
          </div>

          <div className="providers-list">
            <h4>Sync Provider</h4>
            {syncProviders.map(provider => {
              const Icon = getProviderIcon(provider)
              return (
                <label key={provider} className="provider-option">
                  <input
                    type="radio"
                    name="provider"
                    value={provider}
                    checked={selectedProvider === provider}
                    onChange={(e) => handleProviderChange(e.target.value)}
                  />
                  <Icon size={16} />
                  <span className="provider-name">
                    {provider.charAt(0).toUpperCase() + provider.slice(1)}
                  </span>
                </label>
              )
            })}
          </div>

          {showConfig && (
            <div className="sync-config">
              <h4>Configuration</h4>
              
              {selectedProvider === 'git' && (
                <>
                  <div className="form-group">
                    <label>Repository URL</label>
                    <input
                      type="text"
                      value={configForm.repository}
                      onChange={(e) => setConfigForm({ ...configForm, repository: e.target.value })}
                      placeholder="https://github.com/user/repo.git"
                    />
                  </div>

                  <div className="form-group">
                    <label>Branch</label>
                    <input
                      type="text"
                      value={configForm.branch}
                      onChange={(e) => setConfigForm({ ...configForm, branch: e.target.value })}
                      placeholder="main"
                    />
                  </div>

                  <div className="form-group">
                    <label>Username (optional)</label>
                    <input
                      type="text"
                      value={configForm.username}
                      onChange={(e) => setConfigForm({ ...configForm, username: e.target.value })}
                      placeholder="GitHub username"
                    />
                  </div>

                  <div className="form-group">
                    <label>Password/Token</label>
                    <input
                      type="password"
                      value={configForm.password}
                      onChange={(e) => setConfigForm({ ...configForm, password: e.target.value })}
                      placeholder="Personal access token"
                    />
                  </div>
                </>
              )}

              {selectedProvider === 'cloud' && (
                <>
                  <div className="form-group">
                    <label>Endpoint</label>
                    <input
                      type="text"
                      value={configForm.endpoint}
                      onChange={(e) => setConfigForm({ ...configForm, endpoint: e.target.value })}
                      placeholder="s3.amazonaws.com"
                    />
                  </div>

                  <div className="form-group">
                    <label>Access Key</label>
                    <input
                      type="text"
                      value={configForm.username}
                      onChange={(e) => setConfigForm({ ...configForm, username: e.target.value })}
                    />
                  </div>

                  <div className="form-group">
                    <label>Secret Key</label>
                    <input
                      type="password"
                      value={configForm.password}
                      onChange={(e) => setConfigForm({ ...configForm, password: e.target.value })}
                    />
                  </div>

                  <div className="form-group">
                    <label>Path/Prefix</label>
                    <input
                      type="text"
                      value={configForm.path}
                      onChange={(e) => setConfigForm({ ...configForm, path: e.target.value })}
                      placeholder="parsec/settings"
                    />
                  </div>
                </>
              )}

              {selectedProvider === 'local' && (
                <div className="form-group">
                  <label>Local Path</label>
                  <input
                    type="text"
                    value={configForm.path}
                    onChange={(e) => setConfigForm({ ...configForm, path: e.target.value })}
                    placeholder="~/.parsec/settings"
                  />
                </div>
              )}

              <div className="checkbox-group">
                <label>
                  <input
                    type="checkbox"
                    checked={configForm.encrypt}
                    onChange={(e) => setConfigForm({ ...configForm, encrypt: e.target.checked })}
                  />
                  Encrypt settings
                </label>

                <label>
                  <input
                    type="checkbox"
                    checked={configForm.autoSync}
                    onChange={(e) => setConfigForm({ ...configForm, autoSync: e.target.checked })}
                  />
                  Auto-sync
                </label>
              </div>

              {configForm.autoSync && (
                <div className="form-group">
                  <label>Sync Interval (seconds)</label>
                  <input
                    type="number"
                    value={configForm.interval}
                    onChange={(e) => setConfigForm({ ...configForm, interval: parseInt(e.target.value) })}
                    min="60"
                    max="86400"
                  />
                </div>
              )}

              <div className="config-actions">
                <button onClick={handleTestConnection} disabled={testing}>
                  {testing ? 'Testing...' : 'Test Connection'}
                </button>
                <button onClick={handleSaveConfig}>Save Configuration</button>
              </div>

              {testResult && (
                <div className={`test-result ${testResult.success ? 'success' : 'error'}`}>
                  {testResult.success ? <Check size={14} /> : <AlertCircle size={14} />}
                  {testResult.message}
                </div>
              )}
            </div>
          )}

          <div className="sync-history">
            <h4>Sync History</h4>
            <div className="history-list">
              {history.map((item, i) => (
                <div key={i} className="history-item">
                  <span className="history-time">
                    {new Date(item.timestamp).toLocaleString()}
                  </span>
                  <span className={`history-status ${item.success ? 'success' : 'error'}`}>
                    {item.success ? '✓' : '✗'}
                  </span>
                  {item.error && (
                    <span className="history-error">{item.error}</span>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}