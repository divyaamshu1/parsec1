import { useEffect, useRef } from 'react'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import 'xterm/css/xterm.css'
import { useAppStore } from '../../store/appStore'

export default function TerminalComponent() {
  const terminalRef = useRef<HTMLDivElement>(null)
  const term = useRef<Terminal | null>(null)
  const fitAddon = useRef<FitAddon | null>(null)
  const { activeTerminal, writeToTerminal, createTerminal } = useAppStore()

  useEffect(() => {
    if (!terminalRef.current) return

    term.current = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Cascadia Code, monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
      },
    })

    fitAddon.current = new FitAddon()
    term.current.loadAddon(fitAddon.current)
    term.current.open(terminalRef.current)
    fitAddon.current.fit()

    term.current.onData((data) => {
      if (activeTerminal) {
        writeToTerminal(activeTerminal, data)
      }
    })

    createTerminal()

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.current?.fit()
    })
    resizeObserver.observe(terminalRef.current)

    return () => {
      term.current?.dispose()
      resizeObserver.disconnect()
    }
  }, [])

  useEffect(() => {
    if (term.current && activeTerminal) {
      // Clear and write content from store
      term.current.clear()
      // term.current.write(terminals.get(activeTerminal)?.content || '')
    }
  }, [activeTerminal])

  return <div ref={terminalRef} className="terminal" />
}