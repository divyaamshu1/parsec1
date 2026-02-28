import { useState } from 'react'
import { X, File, Plus } from 'lucide-react'
import { useAppStore } from '../../store/appStore'

export default function Tabs() {
  const { files, activeFile, openFile, closeFile } = useAppStore()
  const fileList = Array.from(files.values())

  const handleClose = (e: React.MouseEvent, path: string) => {
    e.stopPropagation()
    closeFile(path)
  }

  const getFileIcon = (filename: string) => {
    const ext = filename.split('.').pop()
    switch (ext) {
      case 'rs': return '🦀'
      case 'ts':
      case 'tsx': return '📘'
      case 'js':
      case 'jsx': return '📜'
      case 'json': return '📋'
      case 'md': return '📝'
      case 'toml': return '⚙️'
      case 'yaml':
      case 'yml': return '⚙️'
      case 'html': return '🌐'
      case 'css': return '🎨'
      default: return <File size={14} />
    }
  }

  return (
    <div className="tabs-container">
      <div className="tabs-list">
        {fileList.map(file => (
          <div
            key={file.path}
            className={`tab ${activeFile === file.path ? 'active' : ''}`}
            onClick={() => openFile(file.path)}
          >
            <span className="tab-icon">{getFileIcon(file.name)}</span>
            <span className="tab-name">{file.name}</span>
            {file.dirty && <span className="tab-dirty">●</span>}
            <button
              className="tab-close"
              onClick={(e) => handleClose(e, file.path)}
            >
              <X size={12} />
            </button>
          </div>
        ))}
        
        <button className="new-tab">
          <Plus size={14} />
        </button>
      </div>
    </div>
  )
}