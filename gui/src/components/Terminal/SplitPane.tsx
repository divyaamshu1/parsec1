import { useEffect, useRef, useState } from 'react'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { useAppStore } from '../../store/appStore'
import 'xterm/css/xterm.css'

export default function TerminalComponent() {
  const terminalRef = useRef<HTMLDivElement>(null)
  const term = useRef<Terminal | null>(null)
  const fitAddon = useRef<FitAddon | null>(null)
  const [terminals, setTerminals] = useState<string[]>(['Terminal 1'])
  const [activeTerminal, setActiveTerminal] = useState(0)
  
  const { writeToTerminal, createTerminal, terminalHeight, setTerminalHeight } = useAppStore()

  useEffect(() => {
    if (!terminalRef.current) return

    term.current = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Cascadia Code, monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
        cursor: '#ffffff',
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
        brightBlack: '#666666',
        brightRed: '#f14c4c',
        brightGreen: '#23d18b',
        brightYellow: '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: '#ffffff'
      }
    })

    fitAddon.current = new FitAddon()
    term.current.loadAddon(fitAddon.current)
    term.current.open(terminalRef.current)
    fitAddon.current.fit()

    term.current.onData((data) => {
      writeToTerminal(`terminal-${activeTerminal}`, data)
    })

    term.current.writeln('Welcome to Parsec Terminal')
    term.current.writeln('$ ')

    createTerminal()

    const handleResize = () => {
      fitAddon.current?.fit()
    }

    window.addEventListener('resize', handleResize)

    return () => {
      term.current?.dispose()
      window.removeEventListener('resize', handleResize)
    }
  }, [])

  useEffect(() => {
    fitAddon.current?.fit()
  }, [terminalHeight])

  const handleNewTerminal = () => {
    setTerminals([...terminals, `Terminal ${terminals.length + 1}`])
    setActiveTerminal(terminals.length)
    createTerminal()
  }

  const handleCloseTerminal = (index: number) => {
    if (terminals.length === 1) return
    const newTerminals = terminals.filter((_, i) => i !== index)
    setTerminals(newTerminals)
    setActiveTerminal(Math.min(index, newTerminals.length - 1))
  }

  const handleResizeStart = (e: React.MouseEvent) => {
    e.preventDefault()
    const startY = e.clientY
    const startHeight = terminalHeight

    const handleMouseMove = (e: MouseEvent) => {
      const delta = startY - e.clientY
      setTerminalHeight(Math.max(100, Math.min(500, startHeight + delta)))
    }

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  return (
    <div className="terminal-container" style={{ height: terminalHeight }}>
      <div className="terminal-tabs">
        {terminals.map((term, i) => (
          <div
            key={i}
            className={`terminal-tab ${activeTerminal === i ? 'active' : ''}`}
            onClick={() => setActiveTerminal(i)}
          >
            <span>{term}</span>
            {terminals.length > 1 && (
              <button onClick={(e) => { e.stopPropagation(); handleCloseTerminal(i); }}>
                ×
              </button>
            )}
          </div>
        ))}
        <button className="new-terminal" onClick={handleNewTerminal}>
          +
        </button>
        <div className="terminal-resize-handle" onMouseDown={handleResizeStart} />
      </div>
      <div ref={terminalRef} className="terminal" />
    </div>
  )
}