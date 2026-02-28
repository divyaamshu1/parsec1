import { useState } from 'react'
import { useAI } from '../../hooks/useAI'
import { 
  Brain, Code, FileText, Bug, Wrench,
  RefreshCw, Copy, Check, Sparkles
} from 'lucide-react'

export default function AIAssistant() {
  const { 
    providers,
    activeProvider,
    activeModel,
    setActiveProvider,
    setActiveModel,
    generateCode,
    explainCode,
    refactorCode,
    findBugs,
    isLoading,
    error
  } = useAI()

  const [mode, setMode] = useState<'generate' | 'explain' | 'refactor' | 'debug'>('generate')
  const [input, setInput] = useState('')
  const [language, setLanguage] = useState('rust')
  const [output, setOutput] = useState('')
  const [copied, setCopied] = useState(false)

  const languages = [
    'rust', 'python', 'javascript', 'typescript', 'go',
    'java', 'c', 'cpp', 'csharp', 'ruby', 'php', 'swift'
  ]

  const handleExecute = async () => {
    if (!input.trim()) return

    setOutput('')
    
    try {
      let result = ''
      switch (mode) {
        case 'generate':
          result = await generateCode(input, language)
          break
        case 'explain':
          result = await explainCode(input)
          break
        case 'refactor':
          result = await refactorCode(input, 'Improve this code')
          break
        case 'debug':
          const bugs = await findBugs(input)
          result = bugs.join('\n')
          break
      }
      setOutput(result)
    } catch (error) {
      setOutput(`Error: ${error}`)
    }
  }

  const copyOutput = () => {
    navigator.clipboard.writeText(output)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const getPlaceholder = () => {
    switch (mode) {
      case 'generate':
        return `Describe what you want to generate in ${language}...`
      case 'explain':
        return 'Paste code to explain...'
      case 'refactor':
        return 'Paste code to refactor...'
      case 'debug':
        return 'Paste code to find bugs...'
    }
  }

  return (
    <div className="ai-panel assistant">
      <div className="panel-header">
        <h3>
          <Brain size={18} /> AI Assistant
        </h3>
      </div>

      <div className="mode-selector">
        <button
          className={mode === 'generate' ? 'active' : ''}
          onClick={() => setMode('generate')}
        >
          <Code size={14} /> Generate
        </button>
        <button
          className={mode === 'explain' ? 'active' : ''}
          onClick={() => setMode('explain')}
        >
          <FileText size={14} /> Explain
        </button>
        <button
          className={mode === 'refactor' ? 'active' : ''}
          onClick={() => setMode('refactor')}
        >
          <Wrench size={14} /> Refactor
        </button>
        <button
          className={mode === 'debug' ? 'active' : ''}
          onClick={() => setMode('debug')}
        >
          <Bug size={14} /> Debug
        </button>
      </div>

      <div className="provider-selector">
        <select 
          value={activeProvider || ''} 
          onChange={(e) => setActiveProvider(e.target.value)}
        >
          <option value="">Select Provider</option>
          {providers.map(p => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </select>

        {activeProvider && (
          <select 
            value={activeModel || ''} 
            onChange={(e) => setActiveModel(e.target.value)}
          >
            {providers
              .find(p => p.id === activeProvider)
              ?.models.map(m => (
                <option key={m} value={m}>{m}</option>
              ))}
          </select>
        )}
      </div>

      {mode === 'generate' && (
        <div className="language-selector">
          <select value={language} onChange={(e) => setLanguage(e.target.value)}>
            {languages.map(lang => (
              <option key={lang} value={lang}>{lang}</option>
            ))}
          </select>
        </div>
      )}

      <div className="input-area">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={getPlaceholder()}
          rows={6}
        />
        <button onClick={handleExecute} disabled={isLoading || !input.trim()}>
          {isLoading ? <RefreshCw size={16} className="spin" /> : <Sparkles size={16} />}
          {mode === 'generate' ? 'Generate' : 
           mode === 'explain' ? 'Explain' :
           mode === 'refactor' ? 'Refactor' : 'Debug'}
        </button>
      </div>

      {error && (
        <div className="error">
          <span className="error-message">{error}</span>
        </div>
      )}

      {output && (
        <div className="output-area">
          <div className="output-header">
            <h4>Output</h4>
            <button onClick={copyOutput}>
              {copied ? <Check size={14} /> : <Copy size={14} />}
            </button>
          </div>
          <pre className="output-content">{output}</pre>
        </div>
      )}
    </div>
  )
}