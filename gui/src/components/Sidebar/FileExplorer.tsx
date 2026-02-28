import { useState, useEffect } from 'react'
import { 
  ChevronRight, ChevronDown, File, Folder, 
  Plus, Search, RefreshCw, MoreVertical 
} from 'lucide-react'
import { useAppStore } from '../../store/appStore'
import { invoke } from '@tauri-apps/api/tauri'

interface TreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: TreeNode[]
  expanded?: boolean
}

export default function FileExplorer() {
  const [tree, setTree] = useState<TreeNode[]>([])
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(false)
  const { openFile, workspacePath, setWorkspace } = useAppStore()

  useEffect(() => {
    if (workspacePath) {
      loadWorkspace()
    }
  }, [workspacePath])

  const loadWorkspace = async () => {
    setLoading(true)
    try {
      const files = await invoke('get_workspace_files') as TreeNode[]
      setTree(files)
    } catch (error) {
      console.error('Failed to load workspace:', error)
    } finally {
      setLoading(false)
    }
  }

  const toggleExpand = (path: string) => {
    const newExpanded = new Set(expanded)
    if (expanded.has(path)) {
      newExpanded.delete(path)
    } else {
      newExpanded.add(path)
    }
    setExpanded(newExpanded)
  }

  const handleFileClick = async (path: string) => {
    await openFile(path)
  }

  const handleOpenWorkspace = async () => {
    const path = await invoke('open_folder_dialog') as string
    if (path) {
      await setWorkspace(path)
    }
  }

  const renderNode = (node: TreeNode, depth: number = 0) => {
    const isExpanded = expanded.has(node.path)

    return (
      <div key={node.path}>
        <div
          className="file-tree-item"
          style={{ paddingLeft: depth * 16 + 8 }}
          onClick={() => {
            if (node.type === 'directory') {
              toggleExpand(node.path)
            } else {
              handleFileClick(node.path)
            }
          }}
        >
          <span className="file-tree-icon">
            {node.type === 'directory' ? (
              isExpanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />
            ) : (
              <File size={16} />
            )}
          </span>
          <span className="file-tree-name">{node.name}</span>
        </div>
        {node.type === 'directory' && isExpanded && node.children && (
          <div className="file-tree-children">
            {node.children.map(child => renderNode(child, depth + 1))}
          </div>
        )}
      </div>
    )
  }

  return (
    <div className="file-explorer">
      <div className="explorer-toolbar">
        <button onClick={handleOpenWorkspace} title="Open Folder">
          <Folder size={16} />
        </button>
        <button onClick={loadWorkspace} disabled={loading}>
          <RefreshCw size={16} className={loading ? 'spin' : ''} />
        </button>
        <button title="New File">
          <Plus size={16} />
        </button>
        <button title="Collapse All">
          <Search size={16} />
        </button>
      </div>

      {!workspacePath ? (
        <div className="empty-workspace">
          <Folder size={48} />
          <h3>No Folder Open</h3>
          <p>Open a folder to start exploring files</p>
          <button onClick={handleOpenWorkspace}>Open Folder</button>
        </div>
      ) : loading ? (
        <div className="loading-files">
          <div className="spinner" />
          <p>Loading files...</p>
        </div>
      ) : (
        <div className="file-tree">
          {tree.map(node => renderNode(node))}
        </div>
      )}
    </div>
  )
}