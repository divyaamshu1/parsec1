import { useState, useEffect } from 'react'
import { useAccessibility } from '../../hooks/useAccessibility'
import { Mic, MicOff, List, Plus, Trash2, Play, Square } from 'lucide-react'

export default function VoiceControl() {
  const { 
    voiceControlEnabled,
    listening,
    wakeWord,
    commands,
    commandHistory,
    toggleVoiceControl,
    setWakeWord,
    addCommand,
    removeCommand,
    startListening,
    stopListening,
    getCommandSuggestions
  } = useAccessibility()

  const [showCommands, setShowCommands] = useState(false)
  const [newCommand, setNewCommand] = useState({
    phrase: '',
    command: '',
    context: 'global'
  })
  const [suggestions, setSuggestions] = useState<string[]>([])

  const contexts = [
    'global',
    'editor',
    'terminal',
    'file-explorer',
    'debug',
    'search',
    'settings'
  ]

  const handleAddCommand = async () => {
    if (!newCommand.phrase || !newCommand.command) return
    
    await addCommand({
      ...newCommand,
      id: Date.now().toString(),
      enabled: true
    })
    
    setNewCommand({ phrase: '', command: '', context: 'global' })
  }

  const handlePhraseChange = async (phrase: string) => {
    setNewCommand({ ...newCommand, phrase })
    const sugg = await getCommandSuggestions(phrase)
    setSuggestions(sugg)
  }

  const handleWakeWordChange = (word: string) => {
    setWakeWord(word)
  }

  return (
    <div className="accessibility-panel voice-control">
      <div className="panel-header">
        <h3>
          <Mic size={18} /> Voice Control
        </h3>
        <label className="toggle">
          <input
            type="checkbox"
            checked={voiceControlEnabled}
            onChange={toggleVoiceControl}
          />
          <span className="toggle-slider"></span>
        </label>
      </div>

      {voiceControlEnabled && (
        <div className="panel-content">
          <div className="status-bar">
            <div className={`status-indicator ${listening ? 'listening' : ''}`}>
              {listening ? (
                <>
                  <Mic size={16} className="pulse" /> Listening...
                </>
              ) : (
                <>
                  <MicOff size={16} /> Not Listening
                </>
              )}
            </div>
            {wakeWord && (
              <div className="wake-word">
                Wake word: "{wakeWord}"
              </div>
            )}
          </div>

          <div className="listening-controls">
            {listening ? (
              <button onClick={stopListening} className="danger">
                <Square size={16} /> Stop Listening
              </button>
            ) : (
              <button onClick={startListening}>
                <Mic size={16} /> Start Listening
              </button>
            )}
          </div>

          <div className="wake-word-settings">
            <h4>Wake Word</h4>
            <input
              type="text"
              value={wakeWord || ''}
              onChange={(e) => handleWakeWordChange(e.target.value)}
              placeholder="e.g., hey parsec"
            />
            <p className="hint">
              Say the wake word before commands to activate
            </p>
          </div>

          <div className="commands-section">
            <div className="section-header">
              <h4>Voice Commands</h4>
              <button onClick={() => setShowCommands(!showCommands)}>
                <Plus size={16} /> Add Command
              </button>
            </div>

            {showCommands && (
              <div className="new-command-form">
                <input
                  type="text"
                  placeholder="Phrase (e.g., open file)"
                  value={newCommand.phrase}
                  onChange={(e) => handlePhraseChange(e.target.value)}
                />
                {suggestions.length > 0 && (
                  <div className="suggestions">
                    {suggestions.map(s => (
                      <button
                        key={s}
                        onClick={() => setNewCommand({ ...newCommand, phrase: s })}
                      >
                        {s}
                      </button>
                    ))}
                  </div>
                )}
                <input
                  type="text"
                  placeholder="Command (e.g., workbench.action.files.open)"
                  value={newCommand.command}
                  onChange={(e) => setNewCommand({ ...newCommand, command: e.target.value })}
                />
                <select
                  value={newCommand.context}
                  onChange={(e) => setNewCommand({ ...newCommand, context: e.target.value })}
                >
                  {contexts.map(ctx => (
                    <option key={ctx} value={ctx}>{ctx}</option>
                  ))}
                </select>
                <div className="form-actions">
                  <button onClick={handleAddCommand}>Add</button>
                  <button onClick={() => setShowCommands(false)}>Cancel</button>
                </div>
              </div>
            )}

            <div className="commands-list">
              {commands.map(cmd => (
                <div key={cmd.id} className="command-item">
                  <div className="command-info">
                    <span className="command-phrase">"{cmd.phrase}"</span>
                    <span className="command-action">→ {cmd.command}</span>
                    <span className="command-context">{cmd.context}</span>
                  </div>
                  <button 
                    className="command-remove"
                    onClick={() => removeCommand(cmd.id)}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
            </div>
          </div>

          <div className="history-section">
            <h4>
              <List size={16} /> Command History
            </h4>
            <div className="history-list">
              {commandHistory.slice(-5).reverse().map((cmd, i) => (
                <div key={i} className="history-item">
                  <span className="history-time">
                    {new Date(cmd.timestamp).toLocaleTimeString()}
                  </span>
                  <span className="history-phrase">"{cmd.phrase}"</span>
                  <span className="history-confidence">
                    {(cmd.confidence * 100).toFixed(0)}%
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}