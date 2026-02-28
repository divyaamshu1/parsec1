import { useState, useCallback } from 'react'

interface DebugSession {
  id: string
  name: string
  state: 'running' | 'paused' | 'stopped'
}

interface Thread {
  id: number
  name: string
  running: boolean
}

interface StackFrame {
  id: number
  name: string
  file: string
  line: number
  column: number
  inLibrary?: boolean
  params?: Array<{ name: string; type: string; value: string }>
  variables?: Array<{ name: string; value: string; type: string }>
}

interface Variable {
  id: string
  name: string
  value: string
  type: string
  children?: Variable[]
}

interface Breakpoint {
  id: string
  file: string
  line: number
  enabled: boolean
  condition?: string
  hitCondition?: string
  logMessage?: string
  verified?: boolean
}

interface Watch {
  id: string
  expression: string
}

export function useDebug() {
  const [sessions, setSessions] = useState<DebugSession[]>([])
  const [activeSession, setActiveSession] = useState<string | null>(null)
  const [threads, setThreads] = useState<Thread[]>([
    { id: 1, name: 'main', running: true },
    { id: 2, name: 'worker', running: true },
    { id: 3, name: 'renderer', running: false }
  ])
  const [activeThread, setActiveThread] = useState<number>(1)
  const [stackFrames, setStackFrames] = useState<StackFrame[]>([
    {
      id: 1,
      name: 'main',
      file: 'src/main.rs',
      line: 42,
      column: 5,
      inLibrary: false,
      params: [
        { name: 'argc', type: 'i32', value: '1' },
        { name: 'argv', type: 'Vec<String>', value: '["program"]' }
      ],
      variables: [
        { name: 'x', value: '42', type: 'i32' },
        { name: 'y', value: '3.14', type: 'f64' }
      ]
    },
    {
      id: 2,
      name: 'calculate',
      file: 'src/lib.rs',
      line: 25,
      column: 10,
      inLibrary: false,
      params: [
        { name: 'input', type: 'i32', value: '42' }
      ]
    },
    {
      id: 3,
      name: 'process',
      file: 'src/lib.rs',
      line: 18,
      column: 8,
      inLibrary: true
    }
  ])
  const [selectedFrame, setSelectedFrame] = useState<number | null>(1)
  const [variables, setVariables] = useState<Variable[]>([
    {
      id: '1',
      name: 'args',
      value: '[...]',
      type: 'Vec<String>',
      children: [
        { id: '1-1', name: '[0]', value: '"program"', type: 'String' },
        { id: '1-2', name: '[1]', value: '"test"', type: 'String' }
      ]
    },
    { id: '2', name: 'counter', value: '42', type: 'i32' },
    {
      id: '3',
      name: 'config',
      value: '{...}',
      type: 'Config',
      children: [
        { id: '3-1', name: 'debug', value: 'true', type: 'bool' },
        { id: '3-2', name: 'port', value: '8080', type: 'u16' }
      ]
    }
  ])
  const [breakpoints, setBreakpoints] = useState<Breakpoint[]>([
    { id: '1', file: 'src/main.rs', line: 42, enabled: true, verified: true },
    { id: '2', file: 'src/lib.rs', line: 25, enabled: true, verified: true },
    { id: '3', file: 'src/lib.rs', line: 30, enabled: false, condition: 'x > 5' }
  ])
  const [watches, setWatches] = useState<Watch[]>([
    { id: '1', expression: 'counter' },
    { id: '2', expression: 'config.debug' }
  ])
  const [isRunning, setIsRunning] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // ==================== Session Management ====================

  const startDebugging = useCallback(async () => {
    setIsLoading(true)
    try {
      setIsRunning(true)
      setSessions(prev => [...prev, { id: Date.now().toString(), name: 'Debug Session', state: 'running' }])
      setActiveSession('1')
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  const stopDebugging = useCallback(async () => {
    setIsRunning(false)
    setActiveSession(null)
  }, [])

  const pauseDebugging = useCallback(async () => {
    setIsRunning(false)
  }, [])

  const resumeDebugging = useCallback(async () => {
    setIsRunning(true)
  }, [])

  // ==================== Thread Management ====================

  const switchThread = useCallback((threadId: number) => {
    setActiveThread(threadId)
    // Update stack frames for this thread
    setSelectedFrame(stackFrames[0]?.id || null)
  }, [stackFrames])

  // ==================== Stack Frame Management ====================

  const selectFrame = useCallback((frameId: number) => {
    setSelectedFrame(frameId)
    // Update variables for this frame
    setVariables(prev => [...prev]) // In real app, fetch variables for this frame
  }, [])

  const getStackTrace = useCallback(async () => {
    return stackFrames
  }, [stackFrames])

  const loadModuleSource = useCallback(async (file: string, line: number) => {
    // In a real app, this would open the file at the specific line
    console.log(`Loading ${file} at line ${line}`)
    return true
  }, [])

  // ==================== Variable Management ====================

  const getVariables = useCallback(async (frameId?: number) => {
    return variables
  }, [variables])

  const setVariableValue = useCallback(async (varId: string, value: string) => {
    setVariables(prev => {
      const updateVariable = (vars: Variable[]): Variable[] => {
        return vars.map(v => {
          if (v.id === varId) {
            return { ...v, value }
          }
          if (v.children) {
            return { ...v, children: updateVariable(v.children) }
          }
          return v
        })
      }
      return updateVariable(prev)
    })
  }, [])

  // ==================== Breakpoint Management ====================

  const setBreakpoint = useCallback(async (file: string, line: number) => {
    const newBreakpoint: Breakpoint = {
      id: Date.now().toString(),
      file,
      line,
      enabled: true,
      verified: true
    }
    setBreakpoints(prev => [...prev, newBreakpoint])
  }, [])

  const removeBreakpoint = useCallback(async (id: string) => {
    setBreakpoints(prev => prev.filter(bp => bp.id !== id))
  }, [])

  const enableBreakpoint = useCallback(async (id: string) => {
    setBreakpoints(prev => prev.map(bp => 
      bp.id === id ? { ...bp, enabled: true } : bp
    ))
  }, [])

  const disableBreakpoint = useCallback(async (id: string) => {
    setBreakpoints(prev => prev.map(bp => 
      bp.id === id ? { ...bp, enabled: false } : bp
    ))
  }, [])

  const setBreakpointCondition = useCallback(async (id: string, condition: string) => {
    setBreakpoints(prev => prev.map(bp => 
      bp.id === id ? { ...bp, condition } : bp
    ))
  }, [])

  const setBreakpointHitCondition = useCallback(async (id: string, hitCondition: string) => {
    setBreakpoints(prev => prev.map(bp => 
      bp.id === id ? { ...bp, hitCondition } : bp
    ))
  }, [])

  const setBreakpointLogMessage = useCallback(async (id: string, message: string) => {
    setBreakpoints(prev => prev.map(bp => 
      bp.id === id ? { ...bp, logMessage: message } : bp
    ))
  }, [])

  // ==================== Watch Management ====================

  const addWatch = useCallback(async (expression: string) => {
    const newWatch: Watch = {
      id: Date.now().toString(),
      expression
    }
    setWatches(prev => [...prev, newWatch])
  }, [])

  const removeWatch = useCallback(async (id: string) => {
    setWatches(prev => prev.filter(w => w.id !== id))
  }, [])

  const updateWatch = useCallback(async (id: string, expression: string) => {
    setWatches(prev => prev.map(w => 
      w.id === id ? { ...w, expression } : w
    ))
  }, [])

  // ==================== Evaluation ====================

  const evaluateExpression = useCallback(async (expression: string, frameId?: number) => {
    // Mock evaluation
    if (expression === 'counter') return '42'
    if (expression === 'config.debug') return 'true'
    return `42 // Result of: ${expression}`
  }, [])

  // ==================== Step Control ====================

  const stepOver = useCallback(async () => {
    console.log('Step over')
  }, [])

  const stepInto = useCallback(async () => {
    console.log('Step into')
  }, [])

  const stepOut = useCallback(async () => {
    console.log('Step out')
  }, [])

  return {
    // State
    sessions,
    activeSession,
    threads,
    activeThread,
    stackFrames,
    selectedFrame,
    variables,
    breakpoints,
    watches,
    isRunning,
    isLoading,
    error,

    // Session
    startDebugging,
    stopDebugging,
    pauseDebugging,
    resumeDebugging,

    // Thread
    switchThread,

    // Stack
    selectFrame,
    getStackTrace,
    loadModuleSource,

    // Variables
    getVariables,
    setVariableValue,

    // Breakpoints
    setBreakpoint,
    removeBreakpoint,
    enableBreakpoint,
    disableBreakpoint,
    setBreakpointCondition,
    setBreakpointHitCondition,
    setBreakpointLogMessage,

    // Watches
    addWatch,
    removeWatch,
    updateWatch,

    // Evaluation
    evaluateExpression,

    // Step
    stepOver,
    stepInto,
    stepOut,
  }
}