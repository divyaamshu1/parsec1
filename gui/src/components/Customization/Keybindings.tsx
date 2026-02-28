import { useState, useEffect } from 'react'
import { useCustomization } from '../../hooks/useCustomization'
import { 
  Keyboard, Search, Plus, Trash2, Edit, Copy, 
  Check, AlertCircle, Download, Upload 
} from 'lucide-react'

export default function Keybindings() {
  const { 
    keybindings,
    keymaps,
    activeKeymap,
    isLoading,
    error,
    loadKeybindings,
    addKeybinding,
    removeKeybinding,
    updateKeybinding,
    exportKeymap,
    importKeymap,
    checkConflicts
  } = useCustomization()

  const [searchQuery, setSearchQuery] = useState('')
  const [filteredBindings, setFilteredBindings] = useState<any[]>([])
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editForm, setEditForm] = useState({
    key: '',
    command: '',
    when: '',
    context: 'global'
  })
  const [conflicts, setConflicts] = useState<any[]>([])
  const [showImport, setShowImport] = useState(false)
  const [importData, setImportData] = useState('')

  useEffect(() => {
    loadKeybindings()
  }, [])

  useEffect(() => {
    if (searchQuery) {
      setFilteredBindings(
        keybindings.filter(b => 
          b.key.toLowerCase().includes(searchQuery.toLowerCase()) ||
          b.command.toLowerCase().includes(searchQuery.toLowerCase()) ||
          b.description?.toLowerCase().includes(searchQuery.toLowerCase())
        )
      )
    } else {
      setFilteredBindings(keybindings)
    }
  }, [searchQuery, keybindings])

  useEffect(() => {
    if (keybindings.length > 0) {
      const conflicts = checkConflicts(keybindings)
      setConflicts(conflicts)
    }
  }, [keybindings, checkConflicts])

  const handleAdd = () => {
    setEditingId('new')
    setEditForm({ key: '', command: '', when: '', context: 'global' })
  }

  const handleEdit = (binding: any) => {
    setEditingId(binding.id)
    setEditForm({
      key: binding.key,
      command: binding.command,
      when: binding.when || '',
      context: binding.context || 'global'
    })
  }

  const handleSave = async () => {
    if (!editForm.key || !editForm.command) return

    if (editingId === 'new') {
      await addKeybinding({
        id: Date.now().toString(),
        ...editForm,
        description: '',
        enabled: true
      })
    } else if (editingId) {
      await updateKeybinding(editingId, editForm)
    }

    setEditingId(null)
  }

  const handleDelete = async (id: string) => {
    if (confirm('Are you sure you want to delete this keybinding?')) {
      await removeKeybinding(id)
    }
  }

  const handleRecordKey = () => {
    // This would open a key recording dialog
    alert('Press any key combination to record...')
  }

  const handleExport = async () => {
    const data = await exportKeymap(activeKeymap || 'default')
    
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `keymap-${activeKeymap}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleImport = async () => {
    try {
      await importKeymap(importData)
      setShowImport(false)
      setImportData('')
    } catch (error) {
      alert('Invalid keymap data')
    }
  }

  const contexts = [
    'global',
    'editor',
    'terminal',
    'explorer',
    'debug',
    'search',
    'settings'
  ]

  const formatKey = (key: string) => {
    return key
      .replace('Ctrl', '⌃')
      .replace('Alt', '⌥')
      .replace('Shift', '⇧')
      .replace('Meta', '⌘')
  }

  return (
    <div className="customization-panel keybindings">
      <div className="panel-header">
        <h3>
          <Keyboard size={18} /> Keybindings
        </h3>
        <div className="header-actions">
          <button onClick={handleExport} title="Export">
            <Download size={16} />
          </button>
          <button onClick={() => setShowImport(true)} title="Import">
            <Upload size={16} />
          </button>
          <button onClick={handleAdd}>
            <Plus size={16} /> Add
          </button>
        </div>
      </div>

      {conflicts.length > 0 && (
        <div className="conflicts-warning">
          <AlertCircle size={16} />
          <span>{conflicts.length} conflicting keybindings found</span>
        </div>
      )}

      <div className="search-bar">
        <Search size={16} />
        <input
          type="text"
          placeholder="Search keybindings..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="keymaps-selector">
        <label>Keymap:</label>
        <select value={activeKeymap || 'default'} onChange={(e) => {/* switch keymap */}}>
          {keymaps.map(km => (
            <option key={km.name} value={km.name}>{km.name}</option>
          ))}
        </select>
      </div>

      <div className="bindings-list">
        {filteredBindings.map(binding => (
          <div key={binding.id} className="binding-item">
            {editingId === binding.id ? (
              <div className="binding-edit">
                <div className="edit-row">
                  <label>Key</label>
                  <div className="key-input">
                    <input
                      type="text"
                      value={editForm.key}
                      onChange={(e) => setEditForm({ ...editForm, key: e.target.value })}
                      placeholder="e.g., Ctrl+S"
                    />
                    <button onClick={handleRecordKey}>Record</button>
                  </div>
                </div>
                <div className="edit-row">
                  <label>Command</label>
                  <input
                    type="text"
                    value={editForm.command}
                    onChange={(e) => setEditForm({ ...editForm, command: e.target.value })}
                    placeholder="e.g., workbench.action.files.save"
                  />
                </div>
                <div className="edit-row">
                  <label>When</label>
                  <input
                    type="text"
                    value={editForm.when}
                    onChange={(e) => setEditForm({ ...editForm, when: e.target.value })}
                    placeholder="e.g., editorFocus"
                  />
                </div>
                <div className="edit-row">
                  <label>Context</label>
                  <select
                    value={editForm.context}
                    onChange={(e) => setEditForm({ ...editForm, context: e.target.value })}
                  >
                    {contexts.map(ctx => (
                      <option key={ctx} value={ctx}>{ctx}</option>
                    ))}
                  </select>
                </div>
                <div className="edit-actions">
                  <button onClick={handleSave}>
                    <Check size={14} /> Save
                  </button>
                  <button onClick={() => setEditingId(null)}>Cancel</button>
                </div>
              </div>
            ) : (
              <>
                <div className="binding-key">
                  <kbd>{formatKey(binding.key)}</kbd>
                </div>
                <div className="binding-info">
                  <div className="binding-command">{binding.command}</div>
                  {binding.description && (
                    <div className="binding-description">{binding.description}</div>
                  )}
                  {binding.when && (
                    <div className="binding-when">when: {binding.when}</div>
                  )}
                </div>
                <div className="binding-actions">
                  <button onClick={() => handleEdit(binding)}>
                    <Edit size={14} />
                  </button>
                  <button onClick={() => handleDelete(binding.id)}>
                    <Trash2 size={14} />
                  </button>
                  <button>
                    <Copy size={14} />
                  </button>
                </div>
              </>
            )}
          </div>
        ))}
      </div>

      {showImport && (
        <div className="modal-overlay">
          <div className="modal">
            <div className="modal-header">
              <h3>Import Keymap</h3>
              <button onClick={() => setShowImport(false)}>✕</button>
            </div>
            <div className="modal-body">
              <textarea
                value={importData}
                onChange={(e) => setImportData(e.target.value)}
                placeholder="Paste keymap JSON here..."
                rows={10}
              />
            </div>
            <div className="modal-footer">
              <button onClick={handleImport}>Import</button>
              <button onClick={() => setShowImport(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}