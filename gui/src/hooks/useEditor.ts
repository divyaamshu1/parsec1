import { useEffect, useRef, useCallback } from 'react'
import { useAppStore } from '../store/appStore'
import { useAIStore } from '../store/aiStore'

export function useEditor() {
  const editorRef = useRef<any>(null)
  const { activeFile, files, updateFileContent, saveFile } = useAppStore()
  const { getCompletion, isGenerating } = useAIStore()
  
  const file = activeFile ? files.get(activeFile) : null

  const handleEditorDidMount = useCallback((editor: any) => {
    editorRef.current = editor
  }, [])

  const handleChange = useCallback((value: string | undefined) => {
    if (value !== undefined && activeFile) {
      updateFileContent(activeFile, value)
    }
  }, [activeFile, updateFileContent])

  const handleSave = useCallback(() => {
    saveFile()
  }, [saveFile])

  const handleAICompletion = useCallback(async () => {
    if (!editorRef.current || !activeFile) return

    const editor = editorRef.current
    const position = editor.getPosition()
    const model = editor.getModel()
    const textUntilPosition = model.getValueInRange({
      startLineNumber: 1,
      startColumn: 1,
      endLineNumber: position.lineNumber,
      endColumn: position.column
    })

    try {
      const completion = await getCompletion(textUntilPosition)
      editor.executeEdits('ai-completion', [{
        range: {
          startLineNumber: position.lineNumber,
          startColumn: position.column,
          endLineNumber: position.lineNumber,
          endColumn: position.column
        },
        text: completion
      }])
    } catch (error) {
      console.error('AI completion failed:', error)
    }
  }, [activeFile, getCompletion])

  const handleFormat = useCallback(async () => {
    if (!editorRef.current) return
    editorRef.current.getAction('editor.action.formatDocument').run()
  }, [])

  const handleFind = useCallback(() => {
    if (!editorRef.current) return
    editorRef.current.getAction('actions.find').run()
  }, [])

  const handleReplace = useCallback(() => {
    if (!editorRef.current) return
    editorRef.current.getAction('editor.action.startFindReplaceAction').run()
  }, [])

  const handleGoToLine = useCallback((line: number) => {
    if (!editorRef.current) return
    editorRef.current.revealLine(line)
    editorRef.current.setPosition({ lineNumber: line, column: 1 })
    editorRef.current.focus()
  }, [])

  const handleSelectAll = useCallback(() => {
    if (!editorRef.current) return
    editorRef.current.getAction('editor.action.selectAll').run()
  }, [])

  const handleCopy = useCallback(() => {
    if (!editorRef.current) return
    document.execCommand('copy')
  }, [])

  const handleCut = useCallback(() => {
    if (!editorRef.current) return
    document.execCommand('cut')
  }, [])

  const handlePaste = useCallback(() => {
    if (!editorRef.current) return
    document.execCommand('paste')
  }, [])

  const handleUndo = useCallback(() => {
    if (!editorRef.current) return
    editorRef.current.trigger('keyboard', 'undo', null)
  }, [])

  const handleRedo = useCallback(() => {
    if (!editorRef.current) return
    editorRef.current.trigger('keyboard', 'redo', null)
  }, [])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        switch (e.key) {
          case 's':
            e.preventDefault()
            handleSave()
            break
          case 'f':
            e.preventDefault()
            handleFind()
            break
          case 'h':
            e.preventDefault()
            handleReplace()
            break
          case 'z':
            if (!e.shiftKey) {
              e.preventDefault()
              handleUndo()
            }
            break
          case 'Z':
            if (e.shiftKey) {
              e.preventDefault()
              handleRedo()
            }
            break
          case ' ':
            if (e.shiftKey) {
              e.preventDefault()
              handleAICompletion()
            }
            break
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleSave, handleFind, handleReplace, handleUndo, handleRedo, handleAICompletion])

  return {
    editorRef,
    file,
    isGenerating,
    handleEditorDidMount,
    handleChange,
    handleSave,
    handleAICompletion,
    handleFormat,
    handleFind,
    handleReplace,
    handleGoToLine,
    handleSelectAll,
    handleCopy,
    handleCut,
    handlePaste,
    handleUndo,
    handleRedo
  }
}