import React, { useState } from 'react'
import { useMobile } from '../../hooks/useMobile'
import { Hammer, Play, Bug, Download, FolderOpen, RefreshCw } from 'lucide-react'

export default function BuildPanel() {
  const { 
    buildAndroid, 
    buildIOS,
    buildProgress,
    isLoading,
    error
  } = useMobile()

  const [projectPath, setProjectPath] = useState('')
  type AndroidSigning = {
    keystore: string
    password: string
    alias: string
  }
  type IosSigning = {
    team: string
    certificate: string
    provisioning: string
  }

  type BuildConfig = {
    configuration: 'debug' | 'release'
    clean: boolean
    signingConfig?: AndroidSigning | IosSigning
  }

  const [buildConfig, setBuildConfig] = useState<BuildConfig>({
    configuration: 'debug',
    clean: false,
    signingConfig: undefined
  })
  const [buildLog, setBuildLog] = useState<string[]>([])

  const handleBuildAndroid = async () => {
    if (!projectPath) return
    
    setBuildLog([])
    try {
      const result = await buildAndroid(projectPath, {
        configuration: buildConfig.configuration,
        clean: buildConfig.clean,
        signingConfig: buildConfig.signingConfig as AndroidSigning | undefined
      })
      setBuildLog(prev => [...prev, ...result.logs.split('\n')])
      
      if (result.success) {
        setBuildLog(prev => [...prev, `✅ Build successful! APK: ${result.outputPath}`])
      } else {
        setBuildLog(prev => [...prev, `❌ Build failed: ${result.error ?? 'unknown error'}`])
      }
    } catch (err) {
      setBuildLog(prev => [...prev, `❌ Build error: ${err}`])
    }
  }

  const handleBuildIOS = async () => {
    if (!projectPath) return
    
    setBuildLog([])
    try {
      const result = await buildIOS(projectPath, {
        configuration: buildConfig.configuration,
        clean: buildConfig.clean,
        signingConfig: buildConfig.signingConfig as IosSigning | undefined
      })
      setBuildLog(prev => [...prev, ...result.logs.split('\n')])
      
      if (result.success) {
        setBuildLog(prev => [...prev, `✅ Build successful! App: ${result.outputPath}`])
      } else {
        setBuildLog(prev => [...prev, `❌ Build failed: ${result.error ?? 'unknown error'}`])
      }
    } catch (err) {
      setBuildLog(prev => [...prev, `❌ Build error: ${err}`])
    }
  }

  const selectProject = async () => {
    // In Tauri, this would open a file dialog
    setProjectPath('/path/to/project')
  }

  return (
    <div className="build-panel">
      <div className="build-header">
        <h3>Build Mobile App</h3>
      </div>

      <div className="project-selector">
        <input
          type="text"
          placeholder="Project Path"
          value={projectPath}
          onChange={(e) => setProjectPath(e.target.value)}
        />
        <button onClick={selectProject}>
          <FolderOpen size={16} />
        </button>
      </div>

      <div className="build-config">
        <label>
          <input
            type="radio"
            name="config"
            value="debug"
            checked={buildConfig.configuration === 'debug'}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setBuildConfig({ ...buildConfig, configuration: e.target.value as 'debug' | 'release' })}
          />
          Debug
        </label>
        <label>
          <input
            type="radio"
            name="config"
            value="release"
            checked={buildConfig.configuration === 'release'}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setBuildConfig({ ...buildConfig, configuration: e.target.value as 'debug' | 'release' })}
          />
          Release
        </label>
        <label>
          <input
            type="checkbox"
            checked={buildConfig.clean}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setBuildConfig({ ...buildConfig, clean: e.target.checked })}
          />
          Clean before build
        </label>
      </div>

      <div className="build-actions">
        <button onClick={handleBuildAndroid} disabled={isLoading || !projectPath}>
          <Hammer size={16} /> Build Android
        </button>
        <button onClick={handleBuildIOS} disabled={isLoading || !projectPath}>
          <Hammer size={16} /> Build iOS
        </button>
        <button disabled>
          <Play size={16} /> Run
        </button>
        <button disabled>
          <Bug size={16} /> Debug
        </button>
      </div>

      {isLoading && (
        <div className="build-progress">
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${buildProgress}%` }} />
          </div>
          <span>{buildProgress}%</span>
        </div>
      )}

      {error && (
        <div className="build-error">
          ❌ {error}
        </div>
      )}

      {buildLog.length > 0 && (
        <div className="build-logs">
          <div className="logs-header">
            <h4>Build Logs</h4>
            <button onClick={() => setBuildLog([])}>
              <RefreshCw size={14} />
            </button>
          </div>
          <div className="logs-content">
            {buildLog.map((line, i) => (
              <pre key={i} className={line.includes('✅') ? 'success' : line.includes('❌') ? 'error' : ''}>
                {line}
              </pre>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}