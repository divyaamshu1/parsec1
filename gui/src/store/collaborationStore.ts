import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/tauri'

interface User {
  id: string
  name: string
  email: string
  avatar?: string
  status: 'online' | 'away' | 'busy' | 'offline'
  currentFile?: string
  cursor?: { line: number; column: number }
}

interface Session {
  id: string
  name: string
  host: User
  participants: User[]
  files: string[]
  createdAt: number
}

interface Comment {
  id: string
  file: string
  line: number
  text: string
  author: User
  createdAt: number
  resolved: boolean
  replies?: Comment[]
}

interface CollaborationState {
  connected: boolean
  user: User | null
  users: User[]
  sessions: Session[]
  activeSession: Session | null
  comments: Comment[]
  following: User | null
  error: string | null

  connect: (server: string) => Promise<void>
  disconnect: () => Promise<void>
  
  createSession: (name: string, password?: string) => Promise<string>
  joinSession: (id: string, password?: string) => Promise<void>
  leaveSession: () => Promise<void>
  
  shareFile: (path: string) => Promise<void>
  unshareFile: (path: string) => Promise<void>
  
  addComment: (file: string, line: number, text: string) => Promise<void>
  resolveComment: (id: string) => Promise<void>
  replyToComment: (id: string, text: string) => Promise<void>
  
  followUser: (userId: string) => Promise<void>
  unfollow: () => Promise<void>
  
  updateCursor: (line: number, column: number) => Promise<void>
  updateSelection: (start: any, end: any) => Promise<void>
  
  startVideoCall: () => Promise<void>
  startVoiceCall: () => Promise<void>
  endCall: () => Promise<void>
  
  sendMessage: (message: string) => Promise<void>
}

export const useCollaborationStore = create<CollaborationState>((set, get) => ({
  connected: false,
  user: null,
  users: [],
  sessions: [],
  activeSession: null,
  comments: [],
  following: null,
  error: null,

  connect: async (server: string) => {
    try {
      const user = await invoke('collab_connect', { server }) as User
      set({ connected: true, user })
      
      // Listen for presence updates
      await invoke('collab_on_presence', (users: User[]) => {
        set({ users })
      })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  disconnect: async () => {
    try {
      await invoke('collab_disconnect')
      set({ connected: false, user: null, users: [], activeSession: null })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  createSession: async (name: string, password?: string) => {
    try {
      const id = await invoke('collab_create_session', { name, password }) as string
      await get().joinSession(id, password)
      return id
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  joinSession: async (id: string, password?: string) => {
    try {
      const session = await invoke('collab_join_session', { id, password }) as Session
      set({ activeSession: session })
      
      // Listen for participant changes
      await invoke('collab_on_participants', (participants: User[]) => {
        if (get().activeSession) {
          set({ activeSession: { ...get().activeSession!, participants } })
        }
      })
      
      // Listen for file shares
      await invoke('collab_on_files', (files: string[]) => {
        if (get().activeSession) {
          set({ activeSession: { ...get().activeSession!, files } })
        }
      })
      
      // Listen for comments
      await invoke('collab_on_comments', (comments: Comment[]) => {
        set({ comments })
      })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  leaveSession: async () => {
    try {
      await invoke('collab_leave_session')
      set({ activeSession: null, comments: [], following: null })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  shareFile: async (path: string) => {
    try {
      await invoke('collab_share_file', { path })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  unshareFile: async (path: string) => {
    try {
      await invoke('collab_unshare_file', { path })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  addComment: async (file: string, line: number, text: string) => {
    try {
      await invoke('collab_add_comment', { file, line, text })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  resolveComment: async (id: string) => {
    try {
      await invoke('collab_resolve_comment', { id })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  replyToComment: async (id: string, text: string) => {
    try {
      await invoke('collab_reply_comment', { id, text })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  followUser: async (userId: string) => {
    try {
      await invoke('collab_follow_user', { userId })
      set({ following: userId })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  unfollow: async () => {
    try {
      await invoke('collab_unfollow')
      set({ following: null })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  updateCursor: async (line: number, column: number) => {
    try {
      await invoke('collab_update_cursor', { line, column })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  updateSelection: async (start: any, end: any) => {
    try {
      await invoke('collab_update_selection', { start, end })
    } catch (error) {
      set({ error: String(error) })
    }
  },

  startVideoCall: async () => {
    try {
      await invoke('collab_start_video')
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  startVoiceCall: async () => {
    try {
      await invoke('collab_start_voice')
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  endCall: async () => {
    try {
      await invoke('collab_end_call')
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },

  sendMessage: async (message: string) => {
    try {
      await invoke('collab_send_message', { message })
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  }
}))