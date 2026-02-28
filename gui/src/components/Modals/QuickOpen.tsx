import { useState, useEffect, useRef } from 'react'
import { useAppStore } from '../../store/appStore'
import { 
  Search, File, Folder, Clock, Star,
  X, ChevronRight, FileText, Image, Music,
  Video, Archive, Code
} from 'lucide-react'

export default function QuickOpen() {
  const { closeModal, openFile } = useAppStore()
  const [search, setSearch] = useState('')
  const [files, setFiles] = useState<any[]>([])
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [recentFiles, setRecentFiles] = useState<any[]>([])
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
    loadRecentFiles()
    searchFiles()
  }, [])

  useEffect(() => {
    const timer = setTimeout(() => {
      searchFiles()
    }, 300)

    return () => clearTimeout(timer)
  }, [search])

  useEffect(() => {
    setSelectedIndex(0)
  }, [files])

  const loadRecentFiles = () => {
    // Mock recent files - would come from store
    setRecentFiles([
      { name: 'main.rs', path: '/src/main.rs', type: 'file' },
      { name: 'app.tsx', path: '/src/app.tsx', type: 'file' },
      { name: 'config.json', path: '/config.json', type: 'file' },
    ])
  }

  const searchFiles = async () => {
    if (!search) {
      setFiles([])
      return
    }

    // Mock search results - would come from backend
    const mockResults = [
      { name: 'main.rs', path: `/src/${search}/main.rs`, type: 'file' },
      { name: 'lib.rs', path: `/src/${search}/lib.rs`, type: 'file' },
      { name: 'mod.rs', path: `/src/${search}/mod.rs`, type: 'file' },
    ]
    setFiles(mockResults)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex(prev => Math.min(prev + 1, (files.length || recentFiles.length) - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex(prev => Math.max(prev - 1, 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      const items = files.length > 0 ? files : recentFiles
      if (items[selectedIndex]) {
        handleSelect(items[selectedIndex])
      }
    } else if (e.key === 'Escape') {
      closeModal()
    }
  }

  const handleSelect = (item: any) => {
    openFile(item.path)
    closeModal()
  }

  const getFileIcon = (filename: string) => {
    const ext = filename.split('.').pop()
    switch (ext) {
      case 'rs': return <Code size={16} />
      case 'ts':
      case 'tsx': return <Code size={16} />
      case 'js':
      case 'jsx': return <Code size={16} />
      case 'json': return <FileText size={16} />
      case 'md': return <FileText size={16} />
      case 'png':
      case 'jpg':
      case 'svg': return <Image size={16} />
      case 'mp3':
      case 'wav': return <Music size={16} />
      case 'mp4':
      case 'mov': return <Video size={16} />
      case 'zip':
      case 'tar': return <Archive size={16} />
      default: return <File size={16} />
    }
  }

  return (
    <div className="modal-overlay" onClick={closeModal}>
      <div className="modal quick-open" onClick={e => e.stopPropagation()}>
        <div className="search-bar">
          <Search size={18} />
          <input
            ref={inputRef}
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Open file (Ctrl+P)"
          />
          <button className="close-btn" onClick={closeModal}>
            <X size={16} />
          </button>
        </div>

        {search ? (
          files.length > 0 ? (
            <div className="results-list">
              {files.map((file, index) => (
                <div
                  key={file.path}
                  className={`result-item ${index === selectedIndex ? 'selected' : ''}`}
                  onClick={() => handleSelect(file)}
                  onMouseEnter={() => setSelectedIndex(index)}
                >
                  {getFileIcon(file.name)}
                  <span className="file-name">{file.name}</span>
                  <span className="file-path">{file.path}</span>
                </div>
              ))}
            </div>
          ) : (
            <div className="no-results">
              <p>No files found</p>
            </div>
          )
        ) : (
          <div className="recent-section">
            <div className="recent-header">
              <Clock size={14} />
              <span>Recent Files</span>
            </div>
            {recentFiles.map((file, index) => (
              <div
                key={file.path}
                className={`result-item ${index === selectedIndex ? 'selected' : ''}`}
                onClick={() => handleSelect(file)}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                {getFileIcon(file.name)}
                <span className="file-name">{file.name}</span>
                <span className="file-path">{file.path}</span>
              </div>
            ))}

            <div className="recent-header" style={{ marginTop: 16 }}>
              <Star size={14} />
              <span>Bookmarks</span>
            </div>
            <div className="empty-message">
              <p>No bookmarks yet</p>
            </div>
          </div>
        )}

        <div className="palette-footer">
          <span>↵ Open</span>
          <span>↑↓ Navigate</span>
          <span>esc Close</span>
        </div>
      </div>
    </div>
  )
}