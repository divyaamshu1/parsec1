import { useState, useEffect } from 'react'
import { useGit } from '../../hooks/useGit'
import { 
  GitBranch, Plus, Trash2, GitMerge, GitPullRequest,
  RefreshCw, Check, ChevronRight, Star, Download 
} from 'lucide-react'

export default function BranchView() {
  const { 
    branches,
    currentBranch,
    isLoading,
    error,
    fetchBranches,
    createBranch,
    deleteBranch,
    checkoutBranch,
    mergeBranch,
    fetchRemoteBranches,
    pushBranch,
    pullBranch
  } = useGit()

  const [showCreate, setShowCreate] = useState(false)
  const [newBranchName, setNewBranchName] = useState('')
  const [newBranchFrom, setNewBranchFrom] = useState(currentBranch || 'main')
  const [showRemote, setShowRemote] = useState(false)
  const [remoteBranches, setRemoteBranches] = useState<any[]>([])
  const [mergeTarget, setMergeTarget] = useState<string | null>(null)

  useEffect(() => {
    fetchBranches()
  }, [])

  const handleCreateBranch = async () => {
    if (!newBranchName) return
    await createBranch(newBranchName, newBranchFrom)
    setShowCreate(false)
    setNewBranchName('')
  }

  const handleCheckout = async (name: string) => {
    await checkoutBranch(name)
  }

  const handleDelete = async (name: string) => {
    if (confirm(`Are you sure you want to delete branch '${name}'?`)) {
      await deleteBranch(name)
    }
  }

  const handleMerge = async (source: string, target: string) => {
    await mergeBranch(source, target)
    setMergeTarget(null)
  }

  const loadRemoteBranches = async () => {
    const remotes = await fetchRemoteBranches()
    setRemoteBranches(remotes)
  }

  const isCurrentBranch = (name: string) => name === currentBranch

  return (
    <div className="git-panel branches">
      <div className="panel-header">
        <h3>
          <GitBranch size={16} /> Branches
        </h3>
        <div className="header-actions">
          <button onClick={() => setShowRemote(!showRemote)}>
            {showRemote ? 'Local' : 'Remote'}
          </button>
          <button onClick={() => setShowCreate(true)}>
            <Plus size={14} /> New
          </button>
          <button onClick={fetchBranches} disabled={isLoading}>
            <RefreshCw size={14} className={isLoading ? 'spin' : ''} />
          </button>
        </div>
      </div>

      {showCreate && (
        <div className="create-branch">
          <input
            type="text"
            value={newBranchName}
            onChange={(e) => setNewBranchName(e.target.value)}
            placeholder="Branch name"
            autoFocus
          />
          <select 
            value={newBranchFrom}
            onChange={(e) => setNewBranchFrom(e.target.value)}
          >
            <option value="">From current</option>
            {branches.map(b => (
              <option key={b.name} value={b.name}>{b.name}</option>
            ))}
          </select>
          <div className="create-actions">
            <button onClick={handleCreateBranch}>Create</button>
            <button onClick={() => setShowCreate(false)}>Cancel</button>
          </div>
        </div>
      )}

      <div className="branches-list">
        {!showRemote ? (
          // Local branches
          branches.map(branch => (
            <div key={branch.name} className="branch-item">
              <div className="branch-info">
                <GitBranch size={14} />
                <span className="branch-name">{branch.name}</span>
                {isCurrentBranch(branch.name) && (
                  <span className="current-badge">current</span>
                )}
              </div>
              <div className="branch-actions">
                {!isCurrentBranch(branch.name) && (
                  <>
                    <button 
                      className="action-btn"
                      onClick={() => handleCheckout(branch.name)}
                      title="Checkout"
                    >
                      <Check size={14} />
                    </button>
                    <button 
                      className="action-btn"
                      onClick={() => setMergeTarget(branch.name)}
                      title="Merge into current"
                    >
                      <GitMerge size={14} />
                    </button>
                  </>
                )}
                <button 
                  className="action-btn danger"
                  onClick={() => handleDelete(branch.name)}
                  title="Delete branch"
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))
        ) : (
          // Remote branches
          <div className="remote-branches">
            <button className="fetch-remote" onClick={loadRemoteBranches}>
              <Download size={14} /> Fetch remote branches
            </button>
            {remoteBranches.map(branch => (
              <div key={branch.name} className="branch-item remote">
                <div className="branch-info">
                  <GitBranch size={14} />
                  <span className="branch-name">{branch.name}</span>
                  <span className="remote-name">{branch.remote}</span>
                </div>
                <div className="branch-actions">
                  <button 
                    className="action-btn"
                    onClick={() => handleCheckout(branch.name)}
                    title="Checkout remote branch"
                  >
                    <Download size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {mergeTarget && (
        <div className="merge-confirm">
          <p>
            Merge <strong>{mergeTarget}</strong> into <strong>{currentBranch}</strong>?
          </p>
          <div className="merge-actions">
            <button onClick={() => handleMerge(mergeTarget, currentBranch || '')}>
              Merge
            </button>
            <button onClick={() => setMergeTarget(null)}>Cancel</button>
          </div>
        </div>
      )}
    </div>
  )
}