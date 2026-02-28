import { useState } from 'react'
import { useAPI } from '../../hooks/useAPI'
import { 
  Folder, File, Plus, Edit, Trash2, Copy, Download,
  Upload, ChevronRight, ChevronDown, Play, Save,
  MoreVertical, FolderPlus
} from 'lucide-react'

export default function Collections() {
  const { 
    collections,
    environments,
    activeEnvironment,
    createCollection,
    deleteCollection,
    createFolder,
    deleteFolder,
    saveRequest,
    deleteRequest,
    runCollection,
    exportCollection,
    importCollection,
    setEnvironment
  } = useAPI()

  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [selectedItem, setSelectedItem] = useState<string | null>(null)
  const [showNewCollection, setShowNewCollection] = useState(false)
  const [newCollectionName, setNewCollectionName] = useState('')
  const [showNewFolder, setShowNewFolder] = useState<string | null>(null)
  const [newFolderName, setNewFolderName] = useState('')
  const [showEnvVars, setShowEnvVars] = useState(false)

  const toggleExpand = (id: string) => {
    const newExpanded = new Set(expanded)
    if (expanded.has(id)) {
      newExpanded.delete(id)
    } else {
      newExpanded.add(id)
    }
    setExpanded(newExpanded)
  }

  const handleCreateCollection = async () => {
    if (!newCollectionName) return
    await createCollection(newCollectionName)
    setNewCollectionName('')
    setShowNewCollection(false)
  }

  const handleCreateFolder = async (collectionId: string) => {
    if (!newFolderName) return
    await createFolder(collectionId, newFolderName)
    setNewFolderName('')
    setShowNewFolder(null)
  }

  const handleExport = async (id: string) => {
    const data = await exportCollection(id)
    
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `collection-${id}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleImport = async () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.json'
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0]
      if (file) {
        const reader = new FileReader()
        reader.onload = async (event) => {
          const data = event.target?.result as string
          await importCollection(data)
        }
        reader.readAsText(file)
      }
    }
    input.click()
  }

  const renderCollection = (collection: any) => {
    const isExpanded = expanded.has(collection.id)

    return (
      <div key={collection.id} className="collection-item">
        <div className="collection-header">
          <button onClick={() => toggleExpand(collection.id)}>
            {isExpanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          </button>
          <Folder size={16} />
          <span className="collection-name">{collection.name}</span>
          <div className="collection-actions">
            <button onClick={() => runCollection(collection.id)} title="Run collection">
              <Play size={14} />
            </button>
            <button onClick={() => setShowNewFolder(collection.id)} title="New folder">
              <FolderPlus size={14} />
            </button>
            <button onClick={() => handleExport(collection.id)} title="Export">
              <Download size={14} />
            </button>
            <button onClick={() => deleteCollection(collection.id)} title="Delete">
              <Trash2 size={14} />
            </button>
          </div>
        </div>

        {isExpanded && (
          <div className="collection-content">
            {collection.folders?.map((folder: any) => renderFolder(folder, collection.id))}
            {collection.requests?.map((req: any) => renderRequest(req))}
          </div>
        )}

        {showNewFolder === collection.id && (
          <div className="new-folder">
            <input
              type="text"
              value={newFolderName}
              onChange={(e) => setNewFolderName(e.target.value)}
              placeholder="Folder name"
              onKeyDown={(e) => e.key === 'Enter' && handleCreateFolder(collection.id)}
            />
            <button onClick={() => handleCreateFolder(collection.id)}>Add</button>
            <button onClick={() => setShowNewFolder(null)}>Cancel</button>
          </div>
        )}
      </div>
    )
  }

  const renderFolder = (folder: any, collectionId: string) => {
    const isExpanded = expanded.has(folder.id)

    return (
      <div key={folder.id} className="folder-item">
        <div className="folder-header">
          <button onClick={() => toggleExpand(folder.id)}>
            {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </button>
          <Folder size={14} />
          <span className="folder-name">{folder.name}</span>
          <div className="folder-actions">
            <button onClick={() => deleteFolder(collectionId, folder.id)}>
              <Trash2 size={12} />
            </button>
          </div>
        </div>

        {isExpanded && (
          <div className="folder-content">
            {folder.requests?.map((req: any) => renderRequest(req))}
          </div>
        )}
      </div>
    )
  }

  const renderRequest = (req: any) => {
    return (
      <div
        key={req.id}
        className={`request-item ${selectedItem === req.id ? 'selected' : ''}`}
        onClick={() => setSelectedItem(req.id)}
      >
        <File size={14} />
        <span className="request-method">{req.method}</span>
        <span className="request-name">{req.name}</span>
        <div className="request-actions">
          <button onClick={(e) => { e.stopPropagation(); saveRequest(req); }}>
            <Save size={12} />
          </button>
          <button onClick={(e) => { e.stopPropagation(); deleteRequest(req.id); }}>
            <Trash2 size={12} />
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="api-client collections">
      <div className="collections-header">
        <h3>Collections</h3>
        <div className="header-actions">
          <button onClick={handleImport} title="Import">
            <Upload size={16} />
          </button>
          <button onClick={() => setShowNewCollection(true)} title="New Collection">
            <Plus size={16} /> New
          </button>
        </div>
      </div>

      {showNewCollection && (
        <div className="new-collection">
          <input
            type="text"
            value={newCollectionName}
            onChange={(e) => setNewCollectionName(e.target.value)}
            placeholder="Collection name"
            onKeyDown={(e) => e.key === 'Enter' && handleCreateCollection()}
          />
          <button onClick={handleCreateCollection}>Create</button>
          <button onClick={() => setShowNewCollection(false)}>Cancel</button>
        </div>
      )}

      <div className="collections-list">
        {collections.map(renderCollection)}
      </div>

      <div className="environments-section">
        <div className="environments-header">
          <h4>Environments</h4>
          <button onClick={() => setShowEnvVars(!showEnvVars)}>
            <Edit size={14} />
          </button>
        </div>

        <select 
          value={activeEnvironment || ''}
          onChange={(e) => setEnvironment(e.target.value)}
        >
          <option value="">No Environment</option>
          {environments.map(env => (
            <option key={env.id} value={env.id}>{env.name}</option>
          ))}
        </select>

        {showEnvVars && activeEnvironment && (
          <div className="environment-variables">
            <h5>Variables</h5>
            {environments
              .find(e => e.id === activeEnvironment)
              ?.variables.map((v: any, i: number) => (
                <div key={i} className="env-var">
                  <span className="var-name">{v.key}</span>
                  <span className="var-value">{v.value}</span>
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  )
}