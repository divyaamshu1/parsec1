import { useState, useEffect, useRef } from 'react'
import { useAppStore } from '../../store/appStore'
import { 
  Search, File, GitBranch, Settings, Terminal, 
  BookOpen, Palette, Cloud, Database, Smartphone,
  Users, Code, Bug, HelpCircle, X
} from 'lucide-react'

export default function CommandPalette() {
  const { closeModal } = useAppStore()
  const [search, setSearch] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  const commands = [
    { id: 'file-new', name: 'File: New File', icon: File, category: 'file' },
    { id: 'file-open', name: 'File: Open...', icon: File, category: 'file' },
    { id: 'file-save', name: 'File: Save', icon: File, category: 'file' },
    { id: 'file-save-all', name: 'File: Save All', icon: File, category: 'file' },
    { id: 'git-commit', name: 'Git: Commit', icon: GitBranch, category: 'git' },
    { id: 'git-push', name: 'Git: Push', icon: GitBranch, category: 'git' },
    { id: 'git-pull', name: 'Git: Pull', icon: GitBranch, category: 'git' },
    { id: 'terminal-new', name: 'Terminal: New Terminal', icon: Terminal, category: 'terminal' },
    { id: 'terminal-clear', name: 'Terminal: Clear', icon: Terminal, category: 'terminal' },
    { id: 'settings-open', name: 'Settings: Open Settings', icon: Settings, category: 'settings' },
    { id: 'settings-theme', name: 'Settings: Change Theme', icon: Palette, category: 'settings' },
    { id: 'settings-keybindings', name: 'Settings: Keybindings', icon: Settings, category: 'settings' },
    { id: 'cloud-deploy', name: 'Cloud: Deploy to AWS', icon: Cloud, category: 'cloud' },
    { id: 'database-connect', name: 'Database: Connect', icon: Database, category: 'database' },
    { id: 'mobile-build', name: 'Mobile: Build Android', icon: Smartphone, category: 'mobile' },
    { id: 'collab-share', name: 'Collaboration: Start Live Share', icon: Users, category: 'collab' },
    { id: 'ai-complete', name: 'AI: Get Completion', icon: Code, category: 'ai' },
    { id: 'ai-chat', name: 'AI: Open Chat', icon: Code, category: 'ai' },
    { id: 'debug-start', name: 'Debug: Start Debugging', icon: Bug, category: 'debug' },
    { id: 'debug-stop', name: 'Debug: Stop', icon: Bug, category: 'debug' },
    { id: 'help-about', name: 'Help: About', icon: HelpCircle, category: 'help' },
    { id: 'help-docs', name: 'Help: Documentation', icon: HelpCircle, category: 'help' },
  ]

  const filteredCommands = commands.filter(cmd =>
    cmd.name.toLowerCase().includes(search.toLowerCase())
  )

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  useEffect(() => {
    setSelectedIndex(0)
  }, [search])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex(prev => Math.min(prev + 1, filteredCommands.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex(prev => Math.max(prev - 1, 0))
    } else if (e.key === 'Enter' && filteredCommands[selectedIndex]) {
      handleSelect(filteredCommands[selectedIndex])
    } else if (e.key === 'Escape') {
      closeModal()
    }
  }

  const handleSelect = (command: typeof commands[0]) => {
    console.log('Executing:', command.id)
    // Execute command
    closeModal()
  }

  return (
    <div className="modal-overlay" onClick={closeModal}>
      <div className="modal command-palette" onClick={e => e.stopPropagation()}>
        <div className="search-bar">
          <Search size={18} />
          <input
            ref={inputRef}
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a command or search..."
          />
          <button className="close-btn" onClick={closeModal}>
            <X size={16} />
          </button>
        </div>

        {filteredCommands.length > 0 ? (
          <div className="commands-list">
            {filteredCommands.map((cmd, index) => {
              const Icon = cmd.icon
              return (
                <div
                  key={cmd.id}
                  className={`command-item ${index === selectedIndex ? 'selected' : ''}`}
                  onClick={() => handleSelect(cmd)}
                  onMouseEnter={() => setSelectedIndex(index)}
                >
                  <Icon size={16} />
                  <span className="command-name">{cmd.name}</span>
                  <span className="command-category">{cmd.category}</span>
                </div>
              )
            })}
          </div>
        ) : (
          <div className="no-results">
            <p>No commands found</p>
          </div>
        )}

        <div className="palette-footer">
          <span>↵ Execute</span>
          <span>↑↓ Navigate</span>
          <span>esc Close</span>
        </div>
      </div>
    </div>
  )
}