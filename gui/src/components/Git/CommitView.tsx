import { useState, useEffect } from 'react'
import { useGit } from '../../hooks/useGit'
import { 
  GitCommit, History, User, Calendar, MessageSquare,
  GitBranch, ArrowLeft, ArrowRight, Search, Download,
  Copy, Check
} from 'lucide-react'

export default function CommitView() {
  const { 
    commits,
    currentBranch,
    isLoading,
    error,
    fetchCommits,
    getCommitDetails,
    getCommitDiff
  } = useGit()

  const [selectedCommit, setSelectedCommit] = useState<string | null>(null)
  const [commitDetails, setCommitDetails] = useState<any>(null)
  const [commitDiff, setCommitDiff] = useState<string>('')
  const [searchQuery, setSearchQuery] = useState('')
  const [copiedHash, setCopiedHash] = useState<string | null>(null)

  useEffect(() => {
    fetchCommits()
  }, [])

  useEffect(() => {
    if (selectedCommit) {
      loadCommitDetails(selectedCommit)
    }
  }, [selectedCommit])

  const loadCommitDetails = async (hash: string) => {
    const details = await getCommitDetails(hash)
    setCommitDetails(details)
    
    const diff = await getCommitDiff(hash)
    setCommitDiff(diff)
  }

  const filteredCommits = commits.filter(commit => 
    commit.message.toLowerCase().includes(searchQuery.toLowerCase()) ||
    commit.author.toLowerCase().includes(searchQuery.toLowerCase()) ||
    commit.hash.includes(searchQuery)
  )

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp)
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))

    if (days === 0) return 'Today'
    if (days === 1) return 'Yesterday'
    if (days < 7) return `${days} days ago`
    return date.toLocaleDateString()
  }

  const copyToClipboard = (text: string, hash: string) => {
    navigator.clipboard.writeText(text)
    setCopiedHash(hash)
    setTimeout(() => setCopiedHash(null), 2000)
  }

  return (
    <div className="git-panel commits">
      <div className="panel-header">
        <h3>
          <GitCommit size={16} /> Commits
        </h3>
        <div className="header-actions">
          <button onClick={() => fetchCommits()} disabled={isLoading}>
            <History size={14} />
          </button>
        </div>
      </div>

      <div className="search-bar">
        <Search size={14} />
        <input
          type="text"
          placeholder="Search commits..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="commits-container">
        <div className="commits-list">
          {filteredCommits.map(commit => (
            <div
              key={commit.hash}
              className={`commit-item ${selectedCommit === commit.hash ? 'selected' : ''}`}
              onClick={() => setSelectedCommit(commit.hash)}
            >
              <div className="commit-header">
                <GitCommit size={12} />
                <span className="commit-hash">{commit.shortHash}</span>
                <button
                  className="copy-btn"
                  onClick={(e) => {
                    e.stopPropagation()
                    copyToClipboard(commit.hash, commit.hash)
                  }}
                >
                  {copiedHash === commit.hash ? <Check size={10} /> : <Copy size={10} />}
                </button>
              </div>
              <div className="commit-message">{commit.message}</div>
              <div className="commit-meta">
                <span className="commit-author">
                  <User size={10} /> {commit.author}
                </span>
                <span className="commit-date">
                  <Calendar size={10} /> {formatDate(commit.timestamp)}
                </span>
              </div>
            </div>
          ))}
        </div>

        {selectedCommit && commitDetails && (
          <div className="commit-details">
            <div className="details-header">
              <h4>Commit Details</h4>
              <div className="detail-actions">
                <button>
                  <ArrowLeft size={14} />
                </button>
                <button>
                  <ArrowRight size={14} />
                </button>
              </div>
            </div>

            <div className="detail-info">
              <div className="detail-row">
                <span className="label">Hash:</span>
                <code>{commitDetails.hash}</code>
              </div>
              <div className="detail-row">
                <span className="label">Author:</span>
                <span>{commitDetails.author}</span>
              </div>
              <div className="detail-row">
                <span className="label">Date:</span>
                <span>{new Date(commitDetails.timestamp).toLocaleString()}</span>
              </div>
              <div className="detail-row">
                <span className="label">Parents:</span>
                <span>{commitDetails.parents}</span>
              </div>
              {commitDetails.branch && (
                <div className="detail-row">
                  <span className="label">Branch:</span>
                  <span className="branch-tag">
                    <GitBranch size={12} /> {commitDetails.branch}
                  </span>
                </div>
              )}
            </div>

            <div className="detail-message">
              <h5>Message</h5>
              <pre>{commitDetails.fullMessage || commitDetails.message}</pre>
            </div>

            {commitDiff && (
              <div className="detail-diff">
                <h5>Changes</h5>
                <pre className="diff-view">{commitDiff}</pre>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}