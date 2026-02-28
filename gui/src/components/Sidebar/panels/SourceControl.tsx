import { useState } from 'react'
import { 
  GitBranch, Plus, RefreshCw, Check, X,
  File, FilePlus, FileMinus, Edit3,
  ChevronDown, ChevronRight
} from 'lucide-react'

export default function SourceControl() {
  const [changes, setChanges] = useState([
    { path: 'src/main.rs', status: 'modified', staged: false },
    { path: 'src/lib.rs', status: 'modified', staged: true },
    { path: 'src/new-file.rs', status: 'added', staged: false },
    { path: 'src/old-file.rs', status: 'deleted', staged: false },
  ])
  const [commitMessage, setCommitMessage] = useState('')
  const [expanded, setExpanded] = useState<Set<string>>(new Set(['changes', 'staged']))
  const [branch] = useState('main')
  const [branches] = useState(['main', 'develop', 'feature/ai', 'bugfix/123'])

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expanded)
    if (expanded.has(section)) {
      newExpanded.delete(section)
    } else {
      newExpanded.add(section)
    }
    setExpanded(newExpanded)
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'added': return <FilePlus size={14} className="added" />
      case 'modified': return <Edit3 size={14} className="modified" />
      case 'deleted': return <FileMinus size={14} className="deleted" />
      default: return <File size={14} />
    }
  }

  const getStatusChar = (status: string) => {
    switch (status) {
      case 'added': return 'A'
      case 'modified': return 'M'
      case 'deleted': return 'D'
      default: return '?'
    }
  }

  const stagedChanges = changes.filter(c => c.staged)
  const unstagedChanges = changes.filter(c => !c.staged)

  const handleStage = (path: string) => {
    setChanges(changes.map(c => 
      c.path === path ? { ...c, staged: true } : c
    ))
  }

  const handleUnstage = (path: string) => {
    setChanges(changes.map(c => 
      c.path === path ? { ...c, staged: false } : c
    ))
  }

  const handleCommit = () => {
    if (!commitMessage.trim() || stagedChanges.length === 0) return
    console.log('Committing:', commitMessage, stagedChanges)
    setCommitMessage('')
    // Clear staged changes
    setChanges(changes.filter(c => !c.staged))
  }

  return (
    <div className="source-control">
      <div className="branch-selector">
        <GitBranch size={14} />
        <select value={branch}>
          {branches.map(b => (
            <option key={b} value={b}>{b}</option>
          ))}
        </select>
        <button title="Fetch">
          <RefreshCw size={14} />
        </button>
      </div>

      <div className="changes-section">
        <div
          className="section-header"
          onClick={() => toggleSection('staged')}
        >
          {expanded.has('staged') ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span>STAGED CHANGES</span>
          <span className="count">{stagedChanges.length}</span>
        </div>

        {expanded.has('staged') && (
          <div className="changes-list">
            {stagedChanges.map(change => (
              <div key={change.path} className="change-item staged">
                <span className="change-status">{getStatusChar(change.status)}</span>
                {getStatusIcon(change.status)}
                <span className="change-path">{change.path}</span>
                <button onClick={() => handleUnstage(change.path)}>
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="changes-section">
        <div
          className="section-header"
          onClick={() => toggleSection('changes')}
        >
          {expanded.has('changes') ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span>CHANGES</span>
          <span className="count">{unstagedChanges.length}</span>
        </div>

        {expanded.has('changes') && (
          <div className="changes-list">
            {unstagedChanges.map(change => (
              <div key={change.path} className="change-item">
                <span className="change-status">{getStatusChar(change.status)}</span>
                {getStatusIcon(change.status)}
                <span className="change-path">{change.path}</span>
                <button onClick={() => handleStage(change.path)}>
                  <Plus size={12} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="commit-section">
        <textarea
          value={commitMessage}
          onChange={(e) => setCommitMessage(e.target.value)}
          placeholder="Commit message"
          rows={3}
        />
        <button 
          className="commit-btn"
          onClick={handleCommit}
          disabled={stagedChanges.length === 0 || !commitMessage.trim()}
        >
          <Check size={14} /> Commit
        </button>
      </div>
    </div>
  )
}