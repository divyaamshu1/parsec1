import Editor from '@monaco-editor/react'
import { useEditor } from '../../hooks/useEditor'
import './editor.css'

export default function EditorComponent() {
  const { 
    file, 
    isGenerating,
    handleEditorDidMount, 
    handleChange 
  } = useEditor()

  if (!file) {
    return (
      <div className="editor-container empty">
        <div className="empty-state">
          <h2>No file open</h2>
          <p>Open a file from the explorer to start editing</p>
        </div>
      </div>
    )
  }

  return (
    <div className="editor-container">
      {isGenerating && (
        <div className="ai-indicator">
          <div className="ai-spinner" />
          <span>AI is generating...</span>
        </div>
      )}
      <Editor
        height="100%"
        defaultLanguage={file.language}
        language={file.language}
        value={file.content}
        onChange={handleChange}
        onMount={handleEditorDidMount}
        theme="vs-dark"
        options={{
          fontSize: 14,
          minimap: { enabled: true },
          wordWrap: 'on',
          lineNumbers: 'on',
          renderWhitespace: 'selection',
          bracketPairColorization: { enabled: true },
          autoIndent: 'full',
          formatOnPaste: true,
          formatOnType: true,
          suggestOnTriggerCharacters: true,
          acceptSuggestionOnEnter: 'on',
          tabCompletion: 'on',
          wordBasedSuggestions: 'currentDocument',
          parameterHints: { enabled: true },
          quickSuggestions: true,
          folding: true,
          foldingHighlight: true,
          links: true,
          mouseWheelZoom: true,
          smoothScrolling: true,
          cursorBlinking: 'blink',
          cursorSmoothCaretAnimation: 'on',
          renderLineHighlight: 'all',
          occurrencesHighlight: 'singleFile',
          selectionHighlight: true,
          colorDecorators: true,
          renderValidationDecorations: 'on'
        }}
      />
    </div>
  )
}