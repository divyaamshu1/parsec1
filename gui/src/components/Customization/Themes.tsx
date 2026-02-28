import { useState, useEffect } from 'react'
import { useCustomization } from '../../hooks/useCustomization'
import { 
  Palette, Sun, Moon, Edit, Copy, Trash2, 
  Download, Upload, Check, Plus
} from 'lucide-react'

export default function Themes() {
  const { 
    themes,
    activeTheme,
    isLoading,
    error,
    loadThemes,
    setActiveTheme,
    createTheme,
    updateTheme,
    deleteTheme,
    exportTheme,
    importTheme
  } = useCustomization()

  const [filter, setFilter] = useState<'all' | 'dark' | 'light'>('all')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editForm, setEditForm] = useState<any>({
    name: '',
    type: 'dark',
    colors: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      primary: '#007acc',
      secondary: '#6c757d',
      accent: '#9cdcfe',
      success: '#6a9955',
      warning: '#cca700',
      error: '#f48771',
      selection: '#264f78',
      border: '#3c3c3c'
    }
  })
  const [showNewForm, setShowNewForm] = useState(false)

  useEffect(() => {
    loadThemes()
  }, [])

  const filteredThemes = themes.filter(theme => 
    filter === 'all' || theme.type === filter
  )

  const handleEdit = (theme: any) => {
    setEditingId(theme.id)
    setEditForm(theme)
  }

  const handleSave = async () => {
    if (editingId) {
      await updateTheme(editingId, editForm)
    }
    setEditingId(null)
  }

  const handleDelete = async (id: string) => {
    if (confirm('Are you sure you want to delete this theme?')) {
      await deleteTheme(id)
    }
  }

  const handleExport = async (id: string) => {
    const data = await exportTheme(id)
    
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `theme-${id}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleCreate = async () => {
    await createTheme(editForm)
    setShowNewForm(false)
    setEditForm({
      name: '',
      type: 'dark',
      colors: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        primary: '#007acc',
        secondary: '#6c757d',
        accent: '#9cdcfe',
        success: '#6a9955',
        warning: '#cca700',
        error: '#f48771',
        selection: '#264f78',
        border: '#3c3c3c'
      }
    })
  }

  return (
    <div className="customization-panel themes">
      <div className="panel-header">
        <h3>
          <Palette size={18} /> Themes
        </h3>
        <div className="header-actions">
          <button onClick={() => setShowNewForm(true)}>
            <Plus size={16} /> New Theme
          </button>
        </div>
      </div>

      <div className="theme-filters">
        <button
          className={filter === 'all' ? 'active' : ''}
          onClick={() => setFilter('all')}
        >
          All
        </button>
        <button
          className={filter === 'dark' ? 'active' : ''}
          onClick={() => setFilter('dark')}
        >
          <Moon size={14} /> Dark
        </button>
        <button
          className={filter === 'light' ? 'active' : ''}
          onClick={() => setFilter('light')}
        >
          <Sun size={14} /> Light
        </button>
      </div>

      <div className="themes-grid">
        {filteredThemes.map(theme => (
          <div
            key={theme.id}
            className={`theme-card ${activeTheme === theme.id ? 'active' : ''}`}
            onClick={() => setActiveTheme(theme.id)}
          >
            <div className="theme-preview">
              <div className="preview-bar" style={{ backgroundColor: theme.colors.primary }} />
              <div className="preview-content">
                <div className="preview-line" style={{ backgroundColor: theme.colors.background }}>
                  <span style={{ color: theme.colors.foreground }}>Text</span>
                  <span style={{ color: theme.colors.accent }}>Accent</span>
                </div>
                <div className="preview-line" style={{ backgroundColor: theme.colors.selection }}>
                  <span style={{ color: theme.colors.foreground }}>Selection</span>
                </div>
                <div className="preview-line">
                  <span style={{ color: theme.colors.success }}>Success</span>
                  <span style={{ color: theme.colors.warning }}>Warning</span>
                  <span style={{ color: theme.colors.error }}>Error</span>
                </div>
              </div>
            </div>
            <div className="theme-info">
              <div className="theme-name">{theme.name}</div>
              <div className="theme-type">{theme.type}</div>
            </div>
            <div className="theme-actions">
              <button onClick={(e) => { e.stopPropagation(); handleEdit(theme); }}>
                <Edit size={14} />
              </button>
              <button onClick={(e) => { e.stopPropagation(); handleExport(theme.id); }}>
                <Download size={14} />
              </button>
              <button onClick={(e) => { e.stopPropagation(); handleDelete(theme.id); }}>
                <Trash2 size={14} />
              </button>
            </div>
            {activeTheme === theme.id && (
              <div className="active-indicator">
                <Check size={14} /> Active
              </div>
            )}
          </div>
        ))}
      </div>

      {(editingId || showNewForm) && (
        <div className="modal-overlay">
          <div className="modal theme-editor">
            <div className="modal-header">
              <h3>{editingId ? 'Edit Theme' : 'New Theme'}</h3>
              <button onClick={() => {
                setEditingId(null)
                setShowNewForm(false)
              }}>✕</button>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label>Name</label>
                <input
                  type="text"
                  value={editForm.name}
                  onChange={(e) => setEditForm({ ...editForm, name: e.target.value })}
                  placeholder="Theme name"
                />
              </div>

              <div className="form-group">
                <label>Type</label>
                <select
                  value={editForm.type}
                  onChange={(e) => setEditForm({ ...editForm, type: e.target.value })}
                >
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                </select>
              </div>

              <h4>Colors</h4>
              <div className="colors-grid">
                {Object.entries(editForm.colors).map(([key, value]) => (
                  <div key={key} className="color-input">
                    <label>{key}</label>
                    <input
                      type="color"
                      value={value as string}
                      onChange={(e) => setEditForm({
                        ...editForm,
                        colors: { ...editForm.colors, [key]: e.target.value }
                      })}
                    />
                  </div>
                ))}
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={editingId ? handleSave : handleCreate}>
                {editingId ? 'Save' : 'Create'}
              </button>
              <button onClick={() => {
                setEditingId(null)
                setShowNewForm(false)
              }}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}