import { useState, useEffect } from 'react'
import { useCustomization } from '../../hooks/useCustomization'
import { 
  Layout, Columns, SplitSquareHorizontal, SplitSquareVertical,
  Maximize, Minimize, Save, Download, Upload, Trash2, Edit, Check
} from 'lucide-react'

export default function Layouts() {
  const { 
    layouts,
    activeLayout,
    isLoading,
    error,
    loadLayouts,
    setActiveLayout,
    createLayout,
    updateLayout,
    deleteLayout,
    exportLayout,
    importLayout,
    saveCurrentLayout
  } = useCustomization()

  const [editingId, setEditingId] = useState<string | null>(null)
  const [editForm, setEditForm] = useState<any>({
    name: '',
    description: '',
    sidebar: {
      visible: true,
      width: 250,
      position: 'left'
    },
    terminal: {
      visible: true,
      height: 200,
      position: 'bottom'
    },
    panels: []
  })
  const [showNewForm, setShowNewForm] = useState(false)
  const [previewMode, setPreviewMode] = useState<'desktop' | 'tablet' | 'mobile'>('desktop')

  useEffect(() => {
    loadLayouts()
  }, [])

  const handleEdit = (layout: any) => {
    setEditingId(layout.id)
    setEditForm(layout)
  }

  const handleSave = async () => {
    if (editingId) {
      await updateLayout(editingId, editForm)
    }
    setEditingId(null)
  }

  const handleDelete = async (id: string) => {
    if (confirm('Are you sure you want to delete this layout?')) {
      await deleteLayout(id)
    }
  }

  const handleExport = async (id: string) => {
    const data = await exportLayout(id)
    
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `layout-${id}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleSaveCurrent = async () => {
    const name = prompt('Enter layout name:')
    if (name) {
      await saveCurrentLayout(name)
    }
  }

  const getPreviewSize = () => {
    switch (previewMode) {
      case 'mobile': return { width: 375, height: 667 }
      case 'tablet': return { width: 768, height: 1024 }
      default: return { width: 1200, height: 800 }
    }
  }

  const renderLayoutPreview = (layout: any) => {
    const { width, height } = getPreviewSize()
    const sidebarWidth = layout.sidebar?.visible ? layout.sidebar.width / 3 : 0
    const terminalHeight = layout.terminal?.visible ? layout.terminal.height / 3 : 0

    return (
      <div 
        className="layout-preview"
        style={{ 
          width: width / 2, 
          height: height / 2,
          backgroundColor: '#1e1e1e'
        }}
      >
        {layout.sidebar?.visible && (
          <div 
            className="preview-sidebar"
            style={{ 
              width: sidebarWidth,
              backgroundColor: '#252526',
              borderRight: '1px solid #3c3c3c'
            }}
          />
        )}
        <div className="preview-main">
          <div className="preview-tabs" style={{ height: 30, backgroundColor: '#2d2d2d' }} />
          <div className="preview-editor" style={{ flex: 1, backgroundColor: '#1e1e1e' }} />
          {layout.terminal?.visible && (
            <div 
              className="preview-terminal"
              style={{ 
                height: terminalHeight,
                backgroundColor: '#1e1e1e',
                borderTop: '1px solid #3c3c3c'
              }}
            />
          )}
        </div>
      </div>
    )
  }

  return (
    <div className="customization-panel layouts">
      <div className="panel-header">
        <h3>
          <Layout size={18} /> Layouts
        </h3>
        <div className="header-actions">
          <button onClick={handleSaveCurrent}>
            <Save size={16} /> Save Current
          </button>
          <button onClick={() => setShowNewForm(true)}>
            Create New
          </button>
        </div>
      </div>

      <div className="preview-controls">
        <button
          className={previewMode === 'desktop' ? 'active' : ''}
          onClick={() => setPreviewMode('desktop')}
        >
          <Maximize size={14} /> Desktop
        </button>
        <button
          className={previewMode === 'tablet' ? 'active' : ''}
          onClick={() => setPreviewMode('tablet')}
        >
          <Columns size={14} /> Tablet
        </button>
        <button
          className={previewMode === 'mobile' ? 'active' : ''}
          onClick={() => setPreviewMode('mobile')}
        >
          <Minimize size={14} /> Mobile
        </button>
      </div>

      <div className="layouts-grid">
        {layouts.map(layout => (
          <div
            key={layout.id}
            className={`layout-card ${activeLayout === layout.id ? 'active' : ''}`}
          >
            {renderLayoutPreview(layout)}
            
            <div className="layout-info">
              <div className="layout-name">{layout.name}</div>
              {layout.description && (
                <div className="layout-description">{layout.description}</div>
              )}
            </div>

            <div className="layout-actions">
              <button 
                onClick={() => setActiveLayout(layout.id)}
                className={activeLayout === layout.id ? 'active' : ''}
              >
                {activeLayout === layout.id ? <Check size={14} /> : 'Apply'}
              </button>
              <button onClick={() => handleEdit(layout)}>
                <Edit size={14} />
              </button>
              <button onClick={() => handleExport(layout.id)}>
                <Download size={14} />
              </button>
              <button onClick={() => handleDelete(layout.id)}>
                <Trash2 size={14} />
              </button>
            </div>
          </div>
        ))}
      </div>

      {(editingId || showNewForm) && (
        <div className="modal-overlay">
          <div className="modal layout-editor">
            <div className="modal-header">
              <h3>{editingId ? 'Edit Layout' : 'New Layout'}</h3>
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
                  placeholder="Layout name"
                />
              </div>

              <div className="form-group">
                <label>Description</label>
                <textarea
                  value={editForm.description}
                  onChange={(e) => setEditForm({ ...editForm, description: e.target.value })}
                  placeholder="Layout description"
                  rows={2}
                />
              </div>

              <h4>Sidebar Settings</h4>
              <div className="settings-group">
                <label>
                  <input
                    type="checkbox"
                    checked={editForm.sidebar?.visible}
                    onChange={(e) => setEditForm({
                      ...editForm,
                      sidebar: { ...editForm.sidebar, visible: e.target.checked }
                    })}
                  />
                  Show Sidebar
                </label>

                {editForm.sidebar?.visible && (
                  <>
                    <div className="form-group">
                      <label>Width (px)</label>
                      <input
                        type="number"
                        value={editForm.sidebar.width}
                        onChange={(e) => setEditForm({
                          ...editForm,
                          sidebar: { ...editForm.sidebar, width: parseInt(e.target.value) }
                        })}
                        min="150"
                        max="400"
                      />
                    </div>

                    <div className="form-group">
                      <label>Position</label>
                      <select
                        value={editForm.sidebar.position}
                        onChange={(e) => setEditForm({
                          ...editForm,
                          sidebar: { ...editForm.sidebar, position: e.target.value }
                        })}
                      >
                        <option value="left">Left</option>
                        <option value="right">Right</option>
                      </select>
                    </div>
                  </>
                )}
              </div>

              <h4>Terminal Settings</h4>
              <div className="settings-group">
                <label>
                  <input
                    type="checkbox"
                    checked={editForm.terminal?.visible}
                    onChange={(e) => setEditForm({
                      ...editForm,
                      terminal: { ...editForm.terminal, visible: e.target.checked }
                    })}
                  />
                  Show Terminal
                </label>

                {editForm.terminal?.visible && (
                  <>
                    <div className="form-group">
                      <label>Height (px)</label>
                      <input
                        type="number"
                        value={editForm.terminal.height}
                        onChange={(e) => setEditForm({
                          ...editForm,
                          terminal: { ...editForm.terminal, height: parseInt(e.target.value) }
                        })}
                        min="100"
                        max="500"
                      />
                    </div>

                    <div className="form-group">
                      <label>Position</label>
                      <select
                        value={editForm.terminal.position}
                        onChange={(e) => setEditForm({
                          ...editForm,
                          terminal: { ...editForm.terminal, position: e.target.value }
                        })}
                      >
                        <option value="bottom">Bottom</option>
                        <option value="top">Top</option>
                        <option value="right">Right</option>
                      </select>
                    </div>
                  </>
                )}
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={editingId ? handleSave : () => createLayout(editForm)}>
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