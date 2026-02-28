import { useEffect, useRef, useCallback } from 'react'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { useAppStore } from '../store/appStore'

export function useTerminal() {
  const terminalRef = useRef<HTMLDivElement>(null)
  const term = useRef<Terminal | null>(null)
  const fitAddon = useRef<FitAddon | null>(null)
  
  const { 
    activeTerminal, 
    terminals, 
    writeToTerminal, 
    createTerminal, 
    clearTerminal,
    setTerminalHeight 
  } = useAppStore()

  const initializeTerminal = useCallback(() => {
    if (!terminalRef.current) return

    term.current = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Cascadia Code, monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
        cursor: '#ffffff',
        selection: '#264f78',
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
      if (activeTerminal) {
        writeToTerminal(activeTerminal, data)
      }
    })

    term.current.onResize((size) => {
      if (activeTerminal) {
        // Send resize to backend
      }
    })

    // Create initial terminal if none exists
    if (!activeTerminal) {
      createTerminal()
    }

    return () => {
      term.current?.dispose()
    }
  }, [activeTerminal, writeToTerminal, createTerminal])

  const writeToTerminalUI = useCallback((data: string) => {
    if (term.current) {
      term.current.write(data)
    }
  }, [])

  const clearTerminalUI = useCallback(() => {
    if (term.current) {
      term.current.clear()
    }
    if (activeTerminal) {
      clearTerminal(activeTerminal)
    }
  }, [activeTerminal, clearTerminal])

  const resizeTerminal = useCallback(() => {
    if (fitAddon.current) {
      fitAddon.current.fit()
      if (term.current) {
        const { cols, rows } = term.current
        // Send resize to backend
      }
    }
  }, [])

  const focusTerminal = useCallback(() => {
    if (term.current) {
      term.current.focus()
    }
  }, [])

  const pasteToTerminal = useCallback((text: string) => {
    if (term.current && activeTerminal) {
      writeToTerminal(activeTerminal, text)
    }
  }, [activeTerminal, writeToTerminal])

  useEffect(() => {
    initializeTerminal()

    const resizeObserver = new ResizeObserver(() => {
      resizeTerminal()
    })

    if (terminalRef.current) {
      resizeObserver.observe(terminalRef.current)
    }

    return () => {
      resizeObserver.disconnect()
    }
  }, [initializeTerminal, resizeTerminal])

  useEffect(() => {
    if (term.current && activeTerminal) {
      const terminal = terminals.get(activeTerminal)
      if (terminal) {
        term.current.clear()
        term.current.write(terminal.content)
      }
    }
  }, [activeTerminal, terminals])

  useEffect(() => {
    const handleResize = () => {
      resizeTerminal()
    }

    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [resizeTerminal])

  return {
    terminalRef,
    writeToTerminal: writeToTerminalUI,
    clearTerminal: clearTerminalUI,
    resizeTerminal,
    focusTerminal,
    pasteToTerminal
  }
}