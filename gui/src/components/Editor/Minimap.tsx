import { useEffect, useRef } from 'react'

interface MinimapProps {
  content: string
  language: string
  width: number
  height: number
  onScroll?: (offset: number) => void
}

export default function Minimap({ content, language, width, height, onScroll }: MinimapProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    if (!canvasRef.current) return

    const canvas = canvasRef.current
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    canvas.width = width
    canvas.height = height

    ctx.fillStyle = '#1e1e1e'
    ctx.fillRect(0, 0, width, height)

    const lines = content.split('\n')
    const lineHeight = 4
    const maxLines = Math.floor(height / lineHeight)

    const visibleLines = lines.slice(0, maxLines)
    visibleLines.forEach((line, i) => {
      const y = i * lineHeight
      const tokens = line.length
      const tokenWidth = Math.min(tokens * 2, width)

      // Simplified syntax highlighting colors
      if (line.trim().startsWith('//') || line.trim().startsWith('#')) {
        ctx.fillStyle = '#6a9955' // comment
      } else if (line.includes('function') || line.includes('fn ')) {
        ctx.fillStyle = '#dcdcaa' // function
      } else if (line.includes('if') || line.includes('for') || line.includes('while')) {
        ctx.fillStyle = '#569cd6' // keyword
      } else {
        ctx.fillStyle = '#9cdcfe' // default
      }

      ctx.fillRect(0, y, tokenWidth, lineHeight - 1)
    })

  }, [content, language, width, height])

  return (
    <canvas
      ref={canvasRef}
      className="minimap"
      style={{ width, height }}
    />
  )
}