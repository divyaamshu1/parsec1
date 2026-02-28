import { useState } from 'react'
import { 
  Play, Square, Pause, SkipForward, 
  CornerDownRight, SkipBack, Variable,
  Layers, Bug, X
} from 'lucide-react'

export default function DebugPanel() {
  const [isRunning, setIsRunning] = useState(false)
  const [breakpoints, setBreakpoints] = useState([
    { file: 'src/main.rs', line: 10, enabled: true },
    { file: 'src/main.rs', line: 15, enabled: true },
    { file: 'src/lib.rs', line: 5, enabled: false },
  ])

  const [variables] = useState([
    { name: 'x', value: '42', type: 'i32' },
    { name: 'y', value: '3.14', type: 'f64' },
    { name: 'message', value: '"hello"', type: 'String' },
  ])

  const [callStack] = useState([
    { name: 'main', file: 'src/main.rs', line: 10 },
    { name: 'calculate', file: 'src/lib.rs', line: 25 },
    { name: 'process', file: 'src/lib.rs', line: 42 },
  ])

  const toggleBreakpoint = (index: number) => {
    setBreakpoints(breakpoints.map((bp, i) =>
      i === index ? { ...bp, enabled: !bp.enabled } : bp
    ))
  }

  return (
    <div className="debug-panel">
      <div className="debug-toolbar">
        <button onClick={() => setIsRunning(!isRunning)}>
          {isRunning ? <Square size={14} /> : <Play size={14} />}
        </button>
        <button disabled={!isRunning}>
          <Pause size={14} />
        </button>
        <button disabled={!isRunning}>
          <SkipForward size={14} />
        </button>
        <button disabled={!isRunning}>
          <CornerDownRight size={14} />
        </button>
        <button disabled={!isRunning}>
          <SkipBack size={14} />
        </button>
      </div>

      <div className="debug-pane">
        <div className="pane-header">
          <Variable size={14} />
          <span>Variables</span>
        </div>
        <div className="variables-list">
          {variables.map((v, i) => (
            <div key={i} className="variable-item">
              <span className="var-name">{v.name}</span>
              <span className="var-value">{v.value}</span>
              <span className="var-type">{v.type}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="debug-pane">
        <div className="pane-header">
          <Layers size={14} />
          <span>Call Stack</span>
        </div>
        <div className="stack-list">
          {callStack.map((frame, i) => (
            <div key={i} className="stack-item">
              <span className="frame-name">{frame.name}</span>
              <span className="frame-location">{frame.file}:{frame.line}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="debug-pane">
        <div className="pane-header">
          <Bug size={14} />
          <span>Breakpoints</span>
        </div>
        <div className="breakpoints-list">
          {breakpoints.map((bp, i) => (
            <div key={i} className="breakpoint-item">
              <input
                type="checkbox"
                checked={bp.enabled}
                onChange={() => toggleBreakpoint(i)}
              />
              <span className="breakpoint-location">
                {bp.file}:{bp.line}
              </span>
              <button>
                <X size={12} />
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}