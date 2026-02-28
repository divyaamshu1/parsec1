import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'
import { AIProvider } from '../types'

interface AIState {
  providers: AIProvider[]
  activeProvider: string | null
  activeModel: string | null
  completions: Array<{ text: string; timestamp: number; provider: string }>
  chatMessages: Array<{ role: 'user' | 'assistant' | 'system'; content: string; timestamp: number }>
  isGenerating: boolean
  error: string | null

  loadProviders: () => Promise<void>
  setActiveProvider: (id: string) => Promise<void>
  setActiveModel: (model: string) => void
  
  getCompletion: (prompt: string, options?: any) => Promise<string>
  getCompletions: (prompt: string, n: number) => Promise<string[]>
  
  sendChatMessage: (message: string) => Promise<string>
  streamChatMessage: (message: string, onToken: (token: string) => void) => Promise<void>
  clearChat: () => void
  
  generateCode: (prompt: string, language: string) => Promise<string>
  explainCode: (code: string) => Promise<string>
  refactorCode: (code: string, instructions: string) => Promise<string>
  findBugs: (code: string) => Promise<string[]>
  
  cancelGeneration: () => Promise<void>
}

export const useAIStore = create<AIState>((set, get) => ({
  providers: [],
  activeProvider: null,
  activeModel: null,
  completions: [],
  chatMessages: [],
  isGenerating: false,
  error: null,

  loadProviders: async () => {
    try {
      const providers = await invoke('get_ai_providers') as AIProvider[]
      set({ 
        providers, 
        activeProvider: providers[0]?.id || null,
        activeModel: providers[0]?.models[0] || null
      })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  setActiveProvider: async (id: string) => {
    const provider = get().providers.find(p => p.id === id)
    set({ 
      activeProvider: id,
      activeModel: provider?.models[0] || null
    })
    await invoke('set_active_ai_provider', { provider: id })
  },

  setActiveModel: (model: string) => {
    set({ activeModel: model })
  },

  getCompletion: async (prompt: string, options?: any) => {
    const { activeProvider, activeModel } = get()
    if (!activeProvider) throw new Error('No AI provider selected')

    set({ isGenerating: true, error: null })
    try {
      const result = await invoke('ai_complete', {
        provider: activeProvider,
        model: activeModel,
        prompt,
        options
      }) as string

      set({ 
        completions: [{ 
          text: result, 
          timestamp: Date.now(),
          provider: activeProvider 
        }, ...get().completions].slice(0, 50),
        isGenerating: false 
      })
      return result
    } catch (error) {
      set({ error: String(error), isGenerating: false })
      throw error
    }
  },

  getCompletions: async (prompt: string, n: number) => {
    const { activeProvider, activeModel } = get()
    if (!activeProvider) throw new Error('No AI provider selected')

    set({ isGenerating: true, error: null })
    try {
      const results = await invoke('ai_completions', {
        provider: activeProvider,
        model: activeModel,
        prompt,
        n
      }) as string[]
      set({ isGenerating: false })
      return results
    } catch (error) {
      set({ error: String(error), isGenerating: false })
      throw error
    }
  },

  sendChatMessage: async (message: string) => {
    const { activeProvider, activeModel, chatMessages } = get()
    if (!activeProvider) throw new Error('No AI provider selected')

    const newMessages = [
      ...chatMessages,
      { role: 'user', content: message, timestamp: Date.now() }
    ]
    set({ chatMessages: newMessages, isGenerating: true, error: null })

    try {
      const response = await invoke('ai_chat', {
        provider: activeProvider,
        model: activeModel,
        messages: newMessages
      }) as string

      set({ 
        chatMessages: [
          ...newMessages,
          { role: 'assistant', content: response, timestamp: Date.now() }
        ],
        isGenerating: false 
      })
      return response
    } catch (error) {
      set({ error: String(error), isGenerating: false })
      throw error
    }
  },

  streamChatMessage: async (message: string, onToken: (token: string) => void) => {
    const { activeProvider, activeModel, chatMessages } = get()
    if (!activeProvider) throw new Error('No AI provider selected')

    const newMessages = [
      ...chatMessages,
      { role: 'user', content: message, timestamp: Date.now() }
    ]
    set({ chatMessages: newMessages, isGenerating: true, error: null })

    try {
      let fullResponse = ''
      await invoke('ai_chat_stream', {
        provider: activeProvider,
        model: activeModel,
        messages: newMessages,
        onToken: (token: string) => {
          fullResponse += token
          onToken(token)
        }
      })

      set({ 
        chatMessages: [
          ...newMessages,
          { role: 'assistant', content: fullResponse, timestamp: Date.now() }
        ],
        isGenerating: false 
      })
    } catch (error) {
      set({ error: String(error), isGenerating: false })
      throw error
    }
  },

  clearChat: () => {
    set({ chatMessages: [] })
  },

  generateCode: async (prompt: string, language: string) => {
    return get().getCompletion(`Generate ${language} code: ${prompt}`)
  },

  explainCode: async (code: string) => {
    return get().getCompletion(`Explain this code:\n\n${code}`)
  },

  refactorCode: async (code: string, instructions: string) => {
    return get().getCompletion(`Refactor this code according to: ${instructions}\n\n${code}`)
  },

  findBugs: async (code: string) => {
    const result = await get().getCompletion(`Find bugs in this code and list them:\n\n${code}`)
    return result.split('\n').filter(line => line.trim().length > 0)
  },

  cancelGeneration: async () => {
    await invoke('ai_cancel')
    set({ isGenerating: false })
  }
}))