import { useEffect } from 'react'
import { useAppStore } from './store/appStore'
import { useAIStore } from './store/aiStore'
import Editor from './components/Editor/Editor'
import Terminal from './components/Terminal/Terminal'
import Sidebar from './components/Sidebar/Sidebar'
import Tabs from './components/Tabs/Tabs'
import StatusBar from './components/StatusBar/StatusBar'
import CommandPalette from './components/Modals/CommandPalette'
import Settings from './components/Modals/Settings'

function App() {
  const { theme, sidebarOpen, terminalHeight, modalOpen, init } = useAppStore()
  const { loadProviders } = useAIStore()

  useEffect(() => {
    init()
    loadProviders()
    document.documentElement.setAttribute('data-theme', theme)
  }, [])

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  return (
    <div className="app">
      {sidebarOpen && <Sidebar />}
      <div className="main">
        <Tabs />
        <Editor />
        {terminalHeight > 0 && <Terminal />}
        <StatusBar />
      </div>
      {modalOpen === 'command' && <CommandPalette />}
      {modalOpen === 'settings' && <Settings />}
    </div>
  )
}

export default App