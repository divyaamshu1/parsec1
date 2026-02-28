import { useState, useEffect } from 'react'
import { useGit } from '../../hooks/useGit'
import { 
  FileText, Plus, Minus, ChevronDown, ChevronRight,
  Download, Copy, Check, FilePlus, FileMinus 
} from 'lucide-react'

export default function DiffViewer() {
  const { 
    getFileDiff,
    getWorkingDirectoryDiff,
    stageFile,
    unstageFile,
    discardChanges
  } = useGit()

  const [files, setFiles] = useState<any[]>([])
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [diffContent, setDiffContent] = useState<string>('')
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [viewMode, setViewMode] = useState<'unified' | 'split'>('unified')
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    loadChangedFiles()
  }, [])

  useEffect(() => {
    if (selectedFile) {
      loadFileDiff(selectedFile)
    }
  }, [selectedFile])

  const loadChangedFiles = async () => {
    const changes = await getWorkingDirectoryDiff()
    setFiles(changes)
  }

  const loadFileDiff = async (file: string) => {
    const diff = await getFileDiff(file)
    setDiffContent(diff)
  }

  const toggleFile = (file: string) => {
    const newExpanded = new Set(expandedFiles)
    if (expandedFiles.has(file)) {
      newExpanded.delete(file)
    } else {
      newExpanded.add(file)
    }
    setExpandedFiles(newExpanded)
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'added': return <FilePlus size={14} className="added" />
      case 'modified': return <FileText size={14} className="modified" />
      case 'deleted': return <FileMinus size={14} className="deleted" />
      default: return <FileText size={14} />
    }
  }

  const getStatusText = (status: string) => {
    switch (status) {
      case 'added': return 'A'
      case 'modified': return 'M'
      case 'deleted': return 'D'
      default: return '?'
    }
  }

  const handleStage = async (file: string) => {
    await stageFile(file)
    await loadChangedFiles()
  }

  const handleUnstage = async (file: string) => {
    await unstageFile(file)
    await loadChangedFiles()
  }

  const handleDiscard = async (file: string) => {
    if (confirm(`Discard changes in ${file}?`)) {
      await discardChanges(file)
      await loadChangedFiles()
    }
  }

  const copyDiff = () => {
    navigator.clipboard.writeText(diffContent)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const renderDiffLine = (line: string, index: number) => {
    const lineClass = line.startsWith('+') ? 'diff-added' :
                     line.startsWith('-') ? 'diff-removed' :
                     line.startsWith('@@') ? 'diff-header' : ''

    return (
      <div key={index} className={`diff-line ${lineClass}`}>
        <span className="line-marker">
          {line.startsWith('+') && <Plus size={12} />}
          {line.startsWith('-') && <Minus size={12} />}
        </span>
        <span className="line-content">{line}</span>
      </div>
    )
  }

  return (
    <div className="git-panel diff">
      <div className="panel-header">
        <h3>Changes</h3>
        <div className="header-actions">
          <button onClick={() => setViewMode(viewMode === 'unified' ? 'split' : 'unified')}>
            {viewMode === 'unified' ? 'Split' : 'Unified'}
          </button>
        </div>
      </div>

      <div className="files-list">
        {files.map(file => (
          <div key={file.path} className="file-item">
            <div 
              className="file-header"
              onClick={() => toggleFile(file.path)}
            >
              {expandedFiles.has(file.path) ? (
                <ChevronDown size={14} />
              ) : (
                <ChevronRight size={14} />
              )}
              {getStatusIcon(file.status)}
              <span className="file-path">{file.path}</span>
              <span className="file-status">{getStatusText(file.status)}</span>
            </div>

            {expandedFiles.has(file.path) && (
              <div className="file-actions">
                <button 
                  className="action-btn"
                  onClick={() => setSelectedFile(file.path)}
                >
                  View Diff
                </button>
                {file.staged ? (
                  <button 
                    className="action-btn"
                    onClick={() => handleUnstage(file.path)}
                  >
                    Unstage
                  </button>
                ) : (
                  <button 
                    className="action-btn"
                    onClick={() => handleStage(file.path)}
                  >
                    Stage
                  </button>
                )}
                <button 
                  className="action-btn danger"
                  onClick={() => handleDiscard(file.path)}
                >
                  Discard
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      {selectedFile && (
        <div className="diff-viewer">
          <div className="diff-header">
            <h4>{selectedFile}</h4>
            <div className="diff-actions">
              <button onClick={copyDiff}>
                {copied ? <Check size={14} /> : <Copy size={14} />}
              </button>
              <button onClick={() => setSelectedFile(null)}>✕</button>
            </div>
          </div>
          <div className="diff-content">
            {diffContent.split('\n').map((line, i) => renderDiffLine(line, i))}
          </div>
        </div>
      )}
    </div>
  )
}