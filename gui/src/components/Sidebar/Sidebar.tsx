import { useState } from 'react'
import { 
  FolderOpen, Search, GitBranch, Puzzle, Bug,
  Users, BookOpen, Palette, Activity, Eye,
  Settings, Cloud, Database, Smartphone, Brain
} from 'lucide-react'
import FileExplorer from './FileExplorer'
import SearchPanel from './panels/SearchPanel'
import SourceControl from './panels/SourceControl'
import Extensions from './panels/Extensions'
import Debug from './panels/Debug'
import AIPanel from './panels/AIPanel'
import CloudPanel from './panels/CloudPanel'
import DatabasePanel from './panels/DatabasePanel'
import MobilePanel from './panels/MobilePanel'
import CollaborationPanel from './panels/CollaborationPanel'
import LearningPanel from './panels/LearningPanel'
import DesignPanel from './panels/DesignPanel'
import MonitoringPanel from './panels/MonitoringPanel'
import AccessibilityPanel from './panels/AccessibilityPanel'

type Panel = 
  | 'explorer' | 'search' | 'source-control' | 'extensions' | 'debug'
  | 'ai' | 'cloud' | 'database' | 'mobile' | 'collaboration'
  | 'learning' | 'design' | 'monitoring' | 'accessibility' | 'settings'

export default function Sidebar() {
  const [activePanel, setActivePanel] = useState<Panel>('explorer')
  const [isCollapsed, setIsCollapsed] = useState(false)

  const panels = [
    { id: 'explorer', icon: FolderOpen, label: 'Explorer' },
    { id: 'search', icon: Search, label: 'Search' },
    { id: 'source-control', icon: GitBranch, label: 'Source Control' },
    { id: 'debug', icon: Bug, label: 'Debug' },
    { id: 'extensions', icon: Puzzle, label: 'Extensions' },
    { id: 'ai', icon: Brain, label: 'AI' },
    { id: 'cloud', icon: Cloud, label: 'Cloud' },
    { id: 'database', icon: Database, label: 'Database' },
    { id: 'mobile', icon: Smartphone, label: 'Mobile' },
    { id: 'collaboration', icon: Users, label: 'Collaboration' },
    { id: 'learning', icon: BookOpen, label: 'Learning' },
    { id: 'design', icon: Palette, label: 'Design' },
    { id: 'monitoring', icon: Activity, label: 'Monitoring' },
    { id: 'accessibility', icon: Eye, label: 'Accessibility' },
    { id: 'settings', icon: Settings, label: 'Settings' },
  ] as const

  const renderPanel = () => {
    switch (activePanel) {
      case 'explorer':
        return <FileExplorer />
      case 'search':
        return <SearchPanel />
      case 'source-control':
        return <SourceControl />
      case 'extensions':
        return <Extensions />
      case 'debug':
        return <Debug />
      case 'ai':
        return <AIPanel />
      case 'cloud':
        return <CloudPanel />
      case 'database':
        return <DatabasePanel />
      case 'mobile':
        return <MobilePanel />
      case 'collaboration':
        return <CollaborationPanel />
      case 'learning':
        return <LearningPanel />
      case 'design':
        return <DesignPanel />
      case 'monitoring':
        return <MonitoringPanel />
      case 'accessibility':
        return <AccessibilityPanel />
      case 'settings':
        return <div>Settings Panel</div>
      default:
        return null
    }
  }

  return (
    <div className={`sidebar ${isCollapsed ? 'collapsed' : ''}`}>
      <div className="sidebar-tabs">
        {panels.map(({ id, icon: Icon, label }) => (
          <button
            key={id}
            className={`sidebar-tab ${activePanel === id ? 'active' : ''}`}
            onClick={() => setActivePanel(id as Panel)}
            title={label}
          >
            <Icon size={20} />
          </button>
        ))}
      </div>

      {!isCollapsed && (
        <div className="sidebar-content">
          <div className="sidebar-header">
            <h3>{panels.find(p => p.id === activePanel)?.label}</h3>
            <button 
              className="collapse-btn"
              onClick={() => setIsCollapsed(true)}
            >
              ◀
            </button>
          </div>
          {renderPanel()}
        </div>
      )}

      {isCollapsed && (
        <button 
          className="expand-btn"
          onClick={() => setIsCollapsed(false)}
        >
          ▶
        </button>
      )}
    </div>
  )
}