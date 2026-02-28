import { useVirtualizer } from '@tanstack/react-virtual'
import { useRef } from 'react'

interface LineNumbersProps {
  lines: string[]
  lineHeight: number
  startLine?: number
}

export default function LineNumbers({ lines, lineHeight, startLine = 1 }: LineNumbersProps) {
  const parentRef = useRef<HTMLDivElement>(null)

  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => lineHeight,
    overscan: 5
  })

  return (
    <div ref={parentRef} className="line-numbers">
      <div style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}>
        {virtualizer.getVirtualItems().map((virtualRow) => (
          <div
            key={virtualRow.key}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`
            }}
            className="line-number"
          >
            {startLine + virtualRow.index}
          </div>
        ))}
      </div>
    </div>
  )
}