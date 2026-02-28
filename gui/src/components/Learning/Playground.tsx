import { useState } from 'react'
import { useLearning } from '../../hooks/useLearning'
import { Play, Save, Plus, X, Download, Copy, Check } from 'lucide-react'
import Editor from '@monaco-editor/react'

export default function Playground() {
  const { 
    playgroundFiles,
    playgroundOutput,
    isLoading,
    executePlayground,
    addPlaygroundFile,
    modifyPlaygroundFile,
    removePlaygroundFile,
    resetPlayground
  } = useLearning()

  const [activeFile, setActiveFile] = useState('main.rs')
  const [newFileName, setNewFileName] = useState('')
  const [showNewFile, setShowNewFile] = useState(false)
  const [copied, setCopied] = useState(false)

  const activeFileContent = playgroundFiles.find(f => f.name === activeFile)?.content || ''

  const handleRun = async () => {
    await executePlayground(activeFile)
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(playgroundOutput)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleAddFile = () => {
    if (!newFileName) return
    
    const ext = newFileName.split('.').pop() || 'txt'
    const language = 
      ext === 'rs' ? 'rust' :
      ext === 'py' ? 'python' :
      ext === 'js' ? 'javascript' :
      ext === 'ts' ? 'typescript' :
      ext === 'go' ? 'go' : 'text'
    
    addPlaygroundFile({
      name: newFileName,
      content: getTemplate(language),
      language
    })
    
    setActiveFile(newFileName)
    setNewFileName('')
    setShowNewFile(false)
  }

  const getTemplate = (language: string): string => {
    switch (language) {
      case 'rust':
        return 'fn main() {\n    println!("Hello, world!");\n}'
      case 'python':
        return 'print("Hello, world!")'
      case 'javascript':
        return 'console.log("Hello, world!");'
      case 'typescript':
        return 'const greeting: string = "Hello, world!";\nconsole.log(greeting);'
      case 'go':
        return 'package main\n\nimport "fmt"\n\nfunc main() {\n    fmt.Println("Hello, world!")\n}'
      default:
        return '// Write your code here'
    }
  }

  return (
    <div className="playground">
      <div className="playground-header">
        <h2>Code Playground</h2>
        <div className="playground-actions">
          <button onClick={handleRun} disabled={isLoading}>
            <Play size={16} /> Run
          </button>
          <button onClick={resetPlayground}>
            Reset
          </button>
        </div>
      </div>

      <div className="playground-main">
        <div className="file-explorer">
          <div className="file-header">
            <h4>Files</h4>
            <button onClick={() => setShowNewFile(!showNewFile)}>
              <Plus size={14} />
            </button>
          </div>

          {showNewFile && (
            <div className="new-file">
              <input
                type="text"
                value={newFileName}
                onChange={(e) => setNewFileName(e.target.value)}
                placeholder="filename.rs"
                onKeyDown={(e) => e.key === 'Enter' && handleAddFile()}
              />
              <button onClick={handleAddFile}>Add</button>
            </div>
          )}

          <div className="file-list">
            {playgroundFiles.map(file => (
              <div
                key={file.name}
                className={`file-item ${activeFile === file.name ? 'active' : ''}`}
                onClick={() => setActiveFile(file.name)}
              >
                <span className="file-name">{file.name}</span>
                <button
                  className="file-remove"
                  onClick={(e) => {
                    e.stopPropagation()
                    if (playgroundFiles.length > 1) {
                      removePlaygroundFile(file.name)
                      if (activeFile === file.name) {
                        setActiveFile(playgroundFiles[0].name)
                      }
                    }
                  }}
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        </div>

        <div className="editor-area">
          <Editor
            height="400px"
            defaultLanguage={playgroundFiles.find(f => f.name === activeFile)?.language || 'text'}
            language={playgroundFiles.find(f => f.name === activeFile)?.language || 'text'}
            value={activeFileContent}
            onChange={(value) => modifyPlaygroundFile(activeFile, value || '')}
            theme="vs-dark"
            options={{
              fontSize: 13,
              minimap: { enabled: false },
              lineNumbers: 'on',
              scrollBeyondLastLine: false,
              wordWrap: 'on'
            }}
          />
        </div>
      </div>

      <div className="output-area">
        <div className="output-header">
          <h4>Output</h4>
          <div className="output-actions">
            <button onClick={handleCopy}>
              {copied ? <Check size={14} /> : <Copy size={14} />}
            </button>
            <button>
              <Download size={14} />
            </button>
          </div>
        </div>
        <pre className="output-content">
          {playgroundOutput || 'Run your code to see output here...'}
        </pre>
      </div>
    </div>
  )
}