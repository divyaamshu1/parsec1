import { useState, useEffect, useRef } from 'react'
import { useDesign } from '../../hooks/useDesign'
import { 
  Move, Square, Circle, Type, Pen, MousePointer,
  ZoomIn, ZoomOut, RotateCw, Save, Download, Copy,
  Bold, Italic, Underline, AlignLeft, AlignCenter, AlignRight
} from 'lucide-react'

export default function SVGEditor() {
  const { 
    svgDocuments,
    activeSVG,
    createSVG,
    loadSVG,
    updateSVG,
    exportSVG,
    optimizeSVG
  } = useDesign()

  const canvasRef = useRef<HTMLDivElement>(null)
  const [selectedTool, setSelectedTool] = useState('select')
  const [zoom, setZoom] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [dragging, setDragging] = useState(false)
  const [selectedElement, setSelectedElement] = useState<string | null>(null)
  const [svgContent, setSvgContent] = useState('')
  const [docName, setDocName] = useState('')
  const [showNewDialog, setShowNewDialog] = useState(false)
  const [newDocWidth, setNewDocWidth] = useState(800)
  const [newDocHeight, setNewDocHeight] = useState(600)

  useEffect(() => {
    if (activeSVG) {
      setSvgContent(activeSVG.svg)
    }
  }, [activeSVG])

  const tools = [
    { id: 'select', icon: MousePointer, label: 'Select' },
    { id: 'move', icon: Move, label: 'Move' },
    { id: 'rect', icon: Square, label: 'Rectangle' },
    { id: 'circle', icon: Circle, label: 'Circle' },
    { id: 'text', icon: Type, label: 'Text' },
    { id: 'pen', icon: Pen, label: 'Pen' }
  ]

  const handleCreateNew = async () => {
    const id = await createSVG(docName, newDocWidth, newDocHeight)
    await loadSVG(id)
    setShowNewDialog(false)
    setDocName('')
  }

  const handleSave = async () => {
    if (!activeSVG) return
    await updateSVG(activeSVG.id, svgContent)
  }

  const handleExport = async (format: 'svg' | 'png' | 'jpg') => {
    if (!activeSVG) return
    
    const data = await exportSVG(activeSVG.id, format)
    
    const blob = new Blob([data], { type: format === 'svg' ? 'image/svg+xml' : `image/${format}` })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${activeSVG.name}.${format}`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleOptimize = async () => {
    if (!activeSVG) return
    
    const optimized = await optimizeSVG(activeSVG.id)
    setSvgContent(optimized)
  }

  const handleToolClick = (toolId: string) => {
    setSelectedTool(toolId)
  }

  const handleZoomIn = () => {
    setZoom(z => Math.min(z * 1.2, 5))
  }

  const handleZoomOut = () => {
    setZoom(z => Math.max(z * 0.8, 0.2))
  }

  const handleResetView = () => {
    setZoom(1)
    setPan({ x: 0, y: 0 })
  }

  return (
    <div className="svg-editor">
      <div className="svg-editor-header">
        <h2>SVG Editor</h2>
        <div className="document-controls">
          <button onClick={() => setShowNewDialog(true)}>New</button>
          <select 
            value={activeSVG?.id || ''}
            onChange={(e) => loadSVG(e.target.value)}
          >
            <option value="">Select Document</option>
            {svgDocuments.map(doc => (
              <option key={doc.id} value={doc.id}>{doc.name}</option>
            ))}
          </select>
        </div>
      </div>

      <div className="svg-editor-main">
        <div className="tools-panel">
          {tools.map(tool => {
            const Icon = tool.icon
            return (
              <button
                key={tool.id}
                className={`tool-button ${selectedTool === tool.id ? 'active' : ''}`}
                onClick={() => handleToolClick(tool.id)}
                title={tool.label}
              >
                <Icon size={20} />
              </button>
            )
          })}

          <div className="tool-divider" />

          <button onClick={handleZoomIn} title="Zoom In">
            <ZoomIn size={20} />
          </button>
          <button onClick={handleZoomOut} title="Zoom Out">
            <ZoomOut size={20} />
          </button>
          <button onClick={handleResetView} title="Reset View">
            <RotateCw size={20} />
          </button>

          <div className="tool-divider" />

          <button onClick={handleSave} title="Save">
            <Save size={20} />
          </button>
          <button onClick={() => handleExport('svg')} title="Export SVG">
            <Download size={20} />
          </button>
          <button onClick={handleOptimize} title="Optimize">
            Optimize
          </button>
        </div>

        <div className="canvas-area">
          <div 
            ref={canvasRef}
            className="canvas"
            style={{
              transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)`,
              transformOrigin: '0 0'
            }}
          >
            {activeSVG ? (
              <div dangerouslySetInnerHTML={{ __html: svgContent }} />
            ) : (
              <div className="empty-canvas">
                <p>Create or select an SVG document to start editing</p>
              </div>
            )}
          </div>
        </div>

        {selectedElement && (
          <div className="properties-panel">
            <h3>Properties</h3>
            <div className="property-group">
              <label>X</label>
              <input type="number" value="0" />
            </div>
            <div className="property-group">
              <label>Y</label>
              <input type="number" value="0" />
            </div>
            <div className="property-group">
              <label>Width</label>
              <input type="number" value="100" />
            </div>
            <div className="property-group">
              <label>Height</label>
              <input type="number" value="100" />
            </div>
            <div className="property-group">
              <label>Fill</label>
              <input type="color" value="#000000" />
            </div>
            <div className="property-group">
              <label>Stroke</label>
              <input type="color" value="#000000" />
            </div>
            <div className="property-group">
              <label>Stroke Width</label>
              <input type="number" value="1" />
            </div>
            <div className="property-group">
              <label>Opacity</label>
              <input type="range" min="0" max="1" step="0.1" value="1" />
            </div>
          </div>
        )}
      </div>

      {showNewDialog && (
        <div className="modal-overlay">
          <div className="modal">
            <div className="modal-header">
              <h3>New SVG Document</h3>
              <button onClick={() => setShowNewDialog(false)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label>Document Name</label>
                <input
                  type="text"
                  value={docName}
                  onChange={(e) => setDocName(e.target.value)}
                  placeholder="My Drawing"
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>Width (px)</label>
                  <input
                    type="number"
                    value={newDocWidth}
                    onChange={(e) => setNewDocWidth(Number(e.target.value))}
                    min="1"
                    max="2000"
                  />
                </div>
                <div className="form-group">
                  <label>Height (px)</label>
                  <input
                    type="number"
                    value={newDocHeight}
                    onChange={(e) => setNewDocHeight(Number(e.target.value))}
                    min="1"
                    max="2000"
                  />
                </div>
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={handleCreateNew}>Create</button>
              <button onClick={() => setShowNewDialog(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}