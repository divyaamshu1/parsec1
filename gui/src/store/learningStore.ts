import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'

export interface Tutorial {
  id: string
  title: string
  description: string
  language: string
  difficulty: 'beginner' | 'intermediate' | 'advanced'
  duration: number
  steps: TutorialStep[]
  completed: boolean
}

export interface TutorialStep {
  id: string
  title: string
  content: string
  code?: string
  solution?: string
  hint?: string
  completed: boolean
  // added to match backend enum, normalized during fetch
  // make optional because some legacy data may omit it before normalization
  stepType?: 'explanation' | 'code' | 'exercise' | 'quiz' | 'challenge' | 'video'
}

interface Snippet {
  id: string
  title: string
  description: string
  code: string
  language: string
  tags: string[]
  author: string
  createdAt: number
  stars: number
}

interface PlaygroundFile {
  name: string
  content: string
  language: string
}

interface LearningState {
  tutorials: Tutorial[]
  activeTutorial: Tutorial | null
  activeStep: number
  snippets: Snippet[]
  snippetCategories: string[]
  playgroundFiles: PlaygroundFile[]
  playgroundOutput: string
  loading: boolean
  error: string | null

  loadTutorials: () => Promise<void>
  loadSnippets: () => Promise<void>
  
  startTutorial: (id: string) => Promise<void>
  completeStep: (stepId: string, code?: string) => Promise<boolean>
  nextStep: () => Promise<TutorialStep | null>
  prevStep: () => Promise<TutorialStep | null>
  
  searchSnippets: (query: string, language?: string) => Promise<Snippet[]>
  getSnippet: (id: string) => Promise<Snippet>
  saveSnippet: (snippet: Omit<Snippet, 'id' | 'author' | 'createdAt' | 'stars'>) => Promise<string>
  starSnippet: (id: string) => Promise<void>
  
  runPlayground: (files: PlaygroundFile[], mainFile: string) => Promise<string>
  clearPlayground: () => void
  addPlaygroundFile: (file: PlaygroundFile) => void
  updatePlaygroundFile: (name: string, content: string) => void
  removePlaygroundFile: (name: string) => void
  
  getCheatSheet: (language: string) => Promise<any>

  // utility used internally to normalize incoming tutorial data
  _normalizeTutorial: (tut: any) => Tutorial
}

export const useLearningStore = create<LearningState>((set, get) => ({
  tutorials: [],
  activeTutorial: null,
  activeStep: 0,
  snippets: [],
  snippetCategories: [],
  playgroundFiles: [
    { name: 'main.rs', content: 'fn main() {\n    println!("Hello, world!");\n}', language: 'rust' }
  ],
  playgroundOutput: '',
  loading: false,
  error: null,

  // helper to convert snake_case step_type to camelCase stepType
  _normalizeTutorial: (tut: any): Tutorial => {
    return {
      ...tut,
      steps: tut.steps.map((s: any) => ({
        ...s,
        stepType: s.stepType || s.step_type || 'explanation'
      }))
    }
  },

  loadTutorials: async () => {
    set({ loading: true })
    try {
      const tutorials = (await invoke('get_tutorials')) as any[]
      const normalized = tutorials.map(get()._normalizeTutorial)
      set({ tutorials: normalized as Tutorial[], loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  loadSnippets: async () => {
    set({ loading: true })
    try {
      const snippets = await invoke('get_snippets') as Snippet[]
      set({ snippets, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  startTutorial: async (id: string) => {
    set({ loading: true })
    try {
      const tut = await invoke('start_tutorial', { id }) as any
      const tutorial = get()._normalizeTutorial(tut) as Tutorial
      set({ activeTutorial: tutorial, activeStep: 0, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  completeStep: async (stepId: string, code?: string) => {
    const { activeTutorial } = get()
    if (!activeTutorial) return false

    try {
      const completed = await invoke('complete_tutorial_step', { 
        tutorialId: activeTutorial.id, 
        stepId, 
        code 
      }) as boolean

      if (completed) {
        const steps = activeTutorial.steps.map(s => 
          s.id === stepId ? { ...s, completed: true } : s
        )
        set({ activeTutorial: { ...activeTutorial, steps } })
      }

      return completed
    } catch (error) {
      set({ error: String(error) })
      return false
    }
  },

  nextStep: async () => {
    const { activeTutorial, activeStep } = get()
    if (!activeTutorial) return null

    const next = activeStep + 1
    if (next < activeTutorial.steps.length) {
      set({ activeStep: next })
      return activeTutorial.steps[next]
    }
    return null
  },

  prevStep: async () => {
    const { activeTutorial, activeStep } = get()
    if (!activeTutorial) return null

    const prev = activeStep - 1
    if (prev >= 0) {
      set({ activeStep: prev })
      return activeTutorial.steps[prev]
    }
    return null
  },

  searchSnippets: async (query: string, language?: string) => {
    try {
      return await invoke('search_snippets', { query, language }) as Snippet[]
    } catch (error) {
      set({ error: String(error) })
      return []
    }
  },

  getSnippet: async (id: string) => {
    try {
      return await invoke('get_snippet', { id }) as Snippet
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  saveSnippet: async (snippet) => {
    try {
      return await invoke('save_snippet', snippet) as string
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  starSnippet: async (id: string) => {
    try {
      await invoke('star_snippet', { id })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  runPlayground: async (files: PlaygroundFile[], mainFile: string) => {
    set({ loading: true })
    try {
      const output = await invoke('run_playground', { files, mainFile }) as string
      set({ playgroundOutput: output, loading: false })
      return output
    } catch (error) {
      set({ error: String(error), loading: false })
      throw error
    }
  },

  clearPlayground: () => {
    set({ playgroundOutput: '' })
  },

  addPlaygroundFile: (file: PlaygroundFile) => {
    set({ playgroundFiles: [...get().playgroundFiles, file] })
  },

  updatePlaygroundFile: (name: string, content: string) => {
    const files = get().playgroundFiles.map(f =>
      f.name === name ? { ...f, content } : f
    )
    set({ playgroundFiles: files })
  },

  removePlaygroundFile: (name: string) => {
    const files = get().playgroundFiles.filter(f => f.name !== name)
    set({ playgroundFiles: files })
  },

  getCheatSheet: async (language: string) => {
    try {
      return await invoke('get_cheat_sheet', { language })
    } catch (error) {
      set({ error: String(error) })
      return null
    }
  }
}))