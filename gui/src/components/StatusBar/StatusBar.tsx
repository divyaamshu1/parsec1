import { useState } from 'react'
import { 
  GitBranch, Wifi, Cpu, Bell, Settings,
  User, Sun, Moon, Cloud, Database
} from 'lucide-react'
import { useAppStore } from '../../store/appStore'

export default function StatusBar() {
  const { theme, toggleTheme, status, setStatus } = useAppStore()
  const [branch] = useState('main')
  const [cursorPos, setCursorPos] = useState({ line: 1, col: 1 })
  const [encoding] = useState('UTF-8')
  const [eol] = useState('LF')
  const [language] = useState('Rust')

  return (
    <div className="status-bar">
      <div className="status-left">
        <div className="status-item" onClick={toggleTheme}>
          {theme === 'dark' ? <Moon size={14} /> : <Sun size={14} />}
        </div>

        <div className="status-item">
          <GitBranch size={14} />
          <span>{branch}</span>
        </div>

        <div className="status-item">
          <div className={`status-indicator ${status}`} />
          <span>{status === 'ready' ? 'Ready' : status}</span>
        </div>

        {status === 'busy' && (
          <div className="status-item">
            <div className="spinner-small" />
          </div>
        )}
      </div>

      <div className="status-right">
        <div className="status-item">
          <Wifi size={14} />
          <span>Connected</span>
        </div>

        <div className="status-item">
          <Cpu size={14} />
          <span>45%</span>
        </div>

        <div className="status-item">
          <Cloud size={14} />
          <span>AWS</span>
        </div>

        <div className="status-item">
          <Database size={14} />
          <span>PostgreSQL</span>
        </div>

        <div className="status-item">
          <span>{language}</span>
        </div>

        <div className="status-item">
          <span>Ln {cursorPos.line}, Col {cursorPos.col}</span>
        </div>

        <div className="status-item">
          <span>{encoding}</span>
        </div>

        <div className="status-item">
          <span>{eol}</span>
        </div>

        <div className="status-item">
          <Bell size={14} />
        </div>

        <div className="status-item">
          <User size={14} />
        </div>

        <div className="status-item">
          <Settings size={14} />
        </div>
      </div>
    </div>
  )
}