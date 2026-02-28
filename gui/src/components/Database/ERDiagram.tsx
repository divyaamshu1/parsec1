import { useEffect, useRef, useState } from 'react'
import { useDatabase } from '../../hooks/useDatabase'
import { ZoomIn, ZoomOut, Maximize, Download } from 'lucide-react'

export default function ERDiagram() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const { tables, getTableSchema } = useDatabase()
  
  const [zoom, setZoom] = useState(1)
  const [offset, setOffset] = useState({ x: 0, y: 0 })
  const [dragging, setDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const [selectedTable, setSelectedTable] = useState<string | null>(null)
  const [relationships, setRelationships] = useState<any[]>([])

  useEffect(() => {
    loadRelationships()
  }, [tables])

  useEffect(() => {
    drawDiagram()
  }, [tables, relationships, zoom, offset, selectedTable])

  const loadRelationships = async () => {
    const rels = []
    for (const table of tables) {
      const schema = await getTableSchema(table.name)
      if (schema?.foreign_keys) {
        for (const fk of schema.foreign_keys) {
          rels.push({
            from: table.name,
            fromColumn: fk.column,
            to: fk.foreign_table,
            toColumn: fk.foreign_column
          })
        }
      }
    }
    setRelationships(rels)
  }

  const drawDiagram = () => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.save()
    ctx.translate(offset.x, offset.y)
    ctx.scale(zoom, zoom)

    // Draw tables
    const tablePositions: Record<string, { x: number; y: number }> = {}
    const cols = Math.ceil(Math.sqrt(tables.length))
    const tableWidth = 200
    const tableHeight = 150
    const padding = 50

    tables.forEach((table, index) => {
      const row = Math.floor(index / cols)
      const col = index % cols
      const x = col * (tableWidth + padding) + padding
      const y = row * (tableHeight + padding) + padding
      tablePositions[table.name] = { x, y }

      // Draw table background
      ctx.fillStyle = selectedTable === table.name ? '#264f78' : '#2d2d2d'
      ctx.fillRect(x, y, tableWidth, tableHeight)
      
      // Draw border
      ctx.strokeStyle = '#3c3c3c'
      ctx.lineWidth = 1
      ctx.strokeRect(x, y, tableWidth, tableHeight)

      // Draw table header
      ctx.fillStyle = '#1e1e1e'
      ctx.fillRect(x, y, tableWidth, 30)
      
      ctx.fillStyle = '#ffffff'
      ctx.font = 'bold 14px monospace'
      ctx.textAlign = 'center'
      ctx.fillText(table.name, x + tableWidth / 2, y + 20)

      // Draw columns
      ctx.fillStyle = '#d4d4d4'
      ctx.font = '12px monospace'
      ctx.textAlign = 'left'
      
      if (table.columns) {
        table.columns.slice(0, 5).forEach((col: any, i: number) => {
          const colY = y + 40 + i * 20
          ctx.fillStyle = col.is_primary_key ? '#6a9955' : '#9cdcfe'
          ctx.fillText(col.name, x + 10, colY)
          ctx.fillStyle = '#888'
          ctx.font = '10px monospace'
          ctx.fillText(col.data_type, x + 120, colY)
          ctx.font = '12px monospace'
        })

        if (table.columns.length > 5) {
          ctx.fillStyle = '#888'
          ctx.font = '12px monospace'
          ctx.fillText(`+${table.columns.length - 5} more...`, x + 10, y + 140)
        }
      }
    })

    // Draw relationships
    ctx.strokeStyle = '#007acc'
    ctx.lineWidth = 2
    ctx.setLineDash([5, 3])

    relationships.forEach(rel => {
      const fromPos = tablePositions[rel.from]
      const toPos = tablePositions[rel.to]
      
      if (fromPos && toPos) {
        ctx.beginPath()
        ctx.moveTo(fromPos.x + tableWidth, fromPos.y + tableHeight / 2)
        ctx.lineTo(toPos.x, toPos.y + tableHeight / 2)
        ctx.stroke()

        // Draw arrow head
        const angle = Math.atan2(toPos.y - fromPos.y, toPos.x - fromPos.x)
        ctx.beginPath()
        ctx.moveTo(toPos.x, toPos.y + tableHeight / 2)
        ctx.lineTo(toPos.x - 10, toPos.y + tableHeight / 2 - 5)
        ctx.lineTo(toPos.x - 10, toPos.y + tableHeight / 2 + 5)
        ctx.closePath()
        ctx.fillStyle = '#007acc'
        ctx.fill()
      }
    })

    ctx.restore()
  }

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? 0.9 : 1.1
    setZoom(prev => Math.max(0.5, Math.min(2, prev * delta)))
  }

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 1 || e.button === 2) {
      setDragging(true)
      setDragStart({ x: e.clientX - offset.x, y: e.clientY - offset.y })
    }
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (dragging) {
      setOffset({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y
      })
    }
  }

  const handleMouseUp = () => {
    setDragging(false)
  }

  const handleZoomIn = () => {
    setZoom(prev => Math.min(2, prev * 1.2))
  }

  const handleZoomOut = () => {
    setZoom(prev => Math.max(0.5, prev * 0.8))
  }

  const handleReset = () => {
    setZoom(1)
    setOffset({ x: 0, y: 0 })
  }

  const handleExport = () => {
    const canvas = canvasRef.current
    if (!canvas) return
    
    const dataUrl = canvas.toDataURL('image/png')
    const link = document.createElement('a')
    link.download = 'er-diagram.png'
    link.href = dataUrl
    link.click()
  }

  return (
    <div className="er-diagram">
      <div className="diagram-toolbar">
        <button onClick={handleZoomIn} title="Zoom In">
          <ZoomIn size={16} />
        </button>
        <button onClick={handleZoomOut} title="Zoom Out">
          <ZoomOut size={16} />
        </button>
        <button onClick={handleReset} title="Reset View">
          <Maximize size={16} />
        </button>
        <button onClick={handleExport} title="Export PNG">
          <Download size={16} />
        </button>
        <span className="zoom-level">{Math.round(zoom * 100)}%</span>
      </div>

      <canvas
        ref={canvasRef}
        width={1200}
        height={800}
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        style={{ cursor: dragging ? 'grabbing' : 'grab' }}
      />
    </div>
  )
}