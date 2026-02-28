import { useState, useCallback, useEffect } from 'react'
import { useLearningStore } from '../store/learningStore'

export function useLearning() {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [tutorialProgress, setTutorialProgress] = useState<number>(0)
  
  const { 
    tutorials,
    activeTutorial,
    activeStep,
    snippets,
    playgroundFiles,
    playgroundOutput,
    loadTutorials,
    loadSnippets,
    startTutorial,
    completeStep,
    nextStep,
    prevStep,
    searchSnippets,
    getSnippet,
    saveSnippet,
    starSnippet,
    runPlayground,
    clearPlayground,
    addPlaygroundFile,
    updatePlaygroundFile,
    removePlaygroundFile,
    getCheatSheet
  } = useLearningStore()

  const fetchTutorials = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      await loadTutorials()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [loadTutorials])

  const fetchSnippets = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      await loadSnippets()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [loadSnippets])

  const beginTutorial = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    setTutorialProgress(0)
    try {
      await startTutorial(id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [startTutorial])

  const completeCurrentStep = useCallback(async (code?: string) => {
    if (!activeTutorial) return false
    
    try {
      const completed = await completeStep(activeTutorial.steps[activeStep].id, code)
      if (completed) {
        setTutorialProgress(((activeStep + 1) / activeTutorial.steps.length) * 100)
      }
      return completed
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return false
    }
  }, [activeTutorial, activeStep, completeStep])

  const goToNextStep = useCallback(async () => {
    try {
      const step = await nextStep()
      if (step) {
        setTutorialProgress(((activeStep + 1) / activeTutorial!.steps.length) * 100)
      }
      return step
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [nextStep, activeStep, activeTutorial])

  const goToPrevStep = useCallback(async () => {
    try {
      const step = await prevStep()
      if (step) {
        setTutorialProgress(((activeStep + 1) / activeTutorial!.steps.length) * 100)
      }
      return step
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [prevStep, activeStep, activeTutorial])

  const searchSnippetLibrary = useCallback(async (query: string, language?: string) => {
    setIsLoading(true)
    try {
      const results = await searchSnippets(query, language)
      return results
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [searchSnippets])

  const saveNewSnippet = useCallback(async (snippet: any) => {
    setIsLoading(true)
    try {
      const id = await saveSnippet(snippet)
      await fetchSnippets()
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [saveSnippet, fetchSnippets])

  const starExistingSnippet = useCallback(async (id: string) => {
    try {
      await starSnippet(id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [starSnippet])

  const executePlayground = useCallback(async (mainFile: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const output = await runPlayground(playgroundFiles, mainFile)
      return output
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [runPlayground, playgroundFiles])

  const addFileToPlayground = useCallback((file: any) => {
    addPlaygroundFile(file)
  }, [addPlaygroundFile])

  const modifyPlaygroundFile = useCallback((name: string, content: string) => {
    updatePlaygroundFile(name, content)
  }, [updatePlaygroundFile])

  const deletePlaygroundFile = useCallback((name: string) => {
    removePlaygroundFile(name)
  }, [removePlaygroundFile])

  const resetPlayground = useCallback(() => {
    clearPlayground()
  }, [clearPlayground])

  const fetchCheatSheet = useCallback(async (language: string) => {
    try {
      return await getCheatSheet(language)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [getCheatSheet])

  const getTutorialById = useCallback((id: string) => {
    return tutorials.find(t => t.id === id)
  }, [tutorials])

  const getSnippetById = useCallback(async (id: string) => {
    try {
      return await getSnippet(id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [getSnippet])

  useEffect(() => {
    fetchTutorials()
    fetchSnippets()
  }, [])

  return {
    // State
    tutorials,
    activeTutorial,
    activeStep,
    snippets,
    playgroundFiles,
    playgroundOutput,
    tutorialProgress,
    isLoading,
    error,
    
    // Actions
    fetchTutorials,
    fetchSnippets,
    beginTutorial,
    completeCurrentStep,
    goToNextStep,
    goToPrevStep,
    searchSnippetLibrary,
    saveNewSnippet,
    starExistingSnippet,
    getSnippetById,
    executePlayground,
    addFileToPlayground,
    modifyPlaygroundFile,
    deletePlaygroundFile,
    resetPlayground,
    fetchCheatSheet,
    getTutorialById
  }
}