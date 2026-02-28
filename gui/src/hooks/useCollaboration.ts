import { useState, useCallback, useEffect } from 'react'

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

interface Message {
  id: string
  userId: string
  userName: string
  text: string
  timestamp: number
}

export function useCollaboration() {
  const [connected, setConnected] = useState(false)
  const [user, setUser] = useState<User | null>(null)
  const [users, setUsers] = useState<User[]>([])
  const [sessions, setSessions] = useState<Session[]>([])
  const [activeSession, setActiveSession] = useState<Session | null>(null)
  const [comments, setComments] = useState<Comment[]>([])
  const [following, setFollowing] = useState<User | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isInCall, setIsInCall] = useState(false)
  const [messages, setMessages] = useState<Message[]>([])
  const [audioEnabled, setAudioEnabled] = useState(true)
  const [videoEnabled, setVideoEnabled] = useState(false)

  // Initialize mock user
  useEffect(() => {
    setUser({
      id: '1',
      name: 'You',
      email: 'you@example.com',
      status: 'online'
    })
  }, [])

  // ==================== Connection Methods ====================

  const connectToServer = useCallback(async (server: string) => {
    try {
      setConnected(true)
      setError(null)
      
      // Mock other users
      setUsers([
        { id: '2', name: 'Alice', email: 'alice@example.com', status: 'online' },
        { id: '3', name: 'Bob', email: 'bob@example.com', status: 'away' },
        { id: '4', name: 'Charlie', email: 'charlie@example.com', status: 'online' },
      ])
      
      return true
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return false
    }
  }, [])

  const disconnectFromServer = useCallback(async () => {
    setConnected(false)
    setActiveSession(null)
    setUsers([])
    setMessages([])
  }, [])

  // ==================== Session Methods ====================

  const createNewSession = useCallback(async (name: string, password?: string) => {
    if (!user) throw new Error('Not logged in')

    const newSession: Session = {
      id: Date.now().toString(),
      name,
      host: user,
      participants: [user],
      files: [],
      createdAt: Date.now()
    }
    
    setSessions(prev => [...prev, newSession])
    setActiveSession(newSession)
    
    // Add to messages
    setMessages(prev => [...prev, {
      id: Date.now().toString(),
      userId: 'system',
      userName: 'System',
      text: `Session "${name}" created`,
      timestamp: Date.now()
    }])
    
    return newSession.id
  }, [user])

  const joinExistingSession = useCallback(async (id: string, password?: string) => {
    const session = sessions.find(s => s.id === id) || {
      id,
      name: `Session ${id}`,
      host: { id: '2', name: 'Alice', email: 'alice@example.com', status: 'online' },
      participants: [
        { id: '2', name: 'Alice', email: 'alice@example.com', status: 'online' }
      ],
      files: [],
      createdAt: Date.now() - 3600000
    }
    
    // Add current user to participants
    if (user && !session.participants.find(p => p.id === user.id)) {
      session.participants.push(user)
    }
    
    setActiveSession(session)
    
    // Add to messages
    setMessages(prev => [...prev, {
      id: Date.now().toString(),
      userId: 'system',
      userName: 'System',
      text: `You joined "${session.name}"`,
      timestamp: Date.now()
    }])
  }, [sessions, user])

  const leaveCurrentSession = useCallback(async () => {
    if (activeSession && user) {
      // Add to messages
      setMessages(prev => [...prev, {
        id: Date.now().toString(),
        userId: 'system',
        userName: 'System',
        text: `You left "${activeSession.name}"`,
        timestamp: Date.now()
      }])
    }
    setActiveSession(null)
  }, [activeSession, user])

  // ==================== File Sharing Methods ====================

  const shareFile = useCallback(async (path: string) => {
    if (!activeSession || !user) return
    
    const fileName = path.split(/[/\\]/).pop() || path
    
    setMessages(prev => [...prev, {
      id: Date.now().toString(),
      userId: 'system',
      userName: 'System',
      text: `${user.name} shared ${fileName}`,
      timestamp: Date.now()
    }])
  }, [activeSession, user])

  const unshareFile = useCallback(async (path: string) => {
    if (!activeSession || !user) return
    // Implementation
  }, [activeSession, user])

  // ==================== Call Methods ====================

  const startCall = useCallback(async (withVideo: boolean = false) => {
    if (!activeSession) throw new Error('No active session')
    
    setIsInCall(true)
    setVideoEnabled(withVideo)
    
    setMessages(prev => [...prev, {
      id: Date.now().toString(),
      userId: 'system',
      userName: 'System',
      text: `Call started${withVideo ? ' with video' : ''}`,
      timestamp: Date.now()
    }])
  }, [activeSession])

  const stopCall = useCallback(async () => {
    setIsInCall(false)
    setAudioEnabled(true)
    setVideoEnabled(false)
    
    setMessages(prev => [...prev, {
      id: Date.now().toString(),
      userId: 'system',
      userName: 'System',
      text: 'Call ended',
      timestamp: Date.now()
    }])
  }, [])

  const toggleAudio = useCallback(() => {
    setAudioEnabled(prev => !prev)
  }, [])

  const toggleVideo = useCallback(() => {
    setVideoEnabled(prev => !prev)
  }, [])

  // ==================== Chat Methods ====================

  const sendChatMessage = useCallback(async (text: string) => {
    if (!user || !activeSession || !text.trim()) return
    
    const newMessage: Message = {
      id: Date.now().toString(),
      userId: user.id,
      userName: user.name,
      text: text.trim(),
      timestamp: Date.now()
    }
    
    setMessages(prev => [...prev, newMessage])
    
    // Simulate reply from other users
    if (activeSession.participants.length > 1) {
      setTimeout(() => {
        const otherUser = activeSession.participants.find(p => p.id !== user.id)
        if (otherUser) {
          setMessages(prev => [...prev, {
            id: (Date.now() + 1).toString(),
            userId: otherUser.id,
            userName: otherUser.name,
            text: `Reply to: ${text.substring(0, 20)}...`,
            timestamp: Date.now()
          }])
        }
      }, 1000)
    }
  }, [user, activeSession])

  // ==================== Comment Methods ====================

  const addComment = useCallback(async (file: string, line: number, text: string) => {
    if (!user) return

    const newComment: Comment = {
      id: Date.now().toString(),
      file,
      line,
      text,
      author: user,
      createdAt: Date.now(),
      resolved: false
    }

    setComments(prev => [...prev, newComment])
  }, [user])

  const resolveComment = useCallback(async (id: string) => {
    setComments(prev => prev.map(comment => 
      comment.id === id ? { ...comment, resolved: true } : comment
    ))
  }, [])

  const replyToComment = useCallback(async (id: string, text: string) => {
    if (!user) return

    const reply: Comment = {
      id: Date.now().toString(),
      file: '',
      line: 0,
      text,
      author: user,
      createdAt: Date.now(),
      resolved: false
    }

    setComments(prev => prev.map(comment => 
      comment.id === id 
        ? { ...comment, replies: [...(comment.replies || []), reply] }
        : comment
    ))
  }, [user])

  // ==================== Presence Methods ====================

  const followUser = useCallback(async (userId: string) => {
    const targetUser = users.find(u => u.id === userId)
    if (targetUser) {
      setFollowing(targetUser)
      
      setMessages(prev => [...prev, {
        id: Date.now().toString(),
        userId: 'system',
        userName: 'System',
        text: `You are now following ${targetUser.name}`,
        timestamp: Date.now()
      }])
    }
  }, [users])

  const unfollow = useCallback(async () => {
    if (following) {
      setMessages(prev => [...prev, {
        id: Date.now().toString(),
        userId: 'system',
        userName: 'System',
        text: `You unfollowed ${following.name}`,
        timestamp: Date.now()
      }])
    }
    setFollowing(null)
  }, [following])

  const updateCursor = useCallback(async (line: number, column: number) => {
    if (!user) return
    // In real app, broadcast to other users
  }, [user])

  const updateSelection = useCallback(async (start: any, end: any) => {
    if (!user) return
    // In real app, broadcast to other users
  }, [user])

  // ==================== Video/Audio Methods ====================

  const startVideoCall = useCallback(async () => {
    return startCall(true)
  }, [startCall])

  const startVoiceCall = useCallback(async () => {
    return startCall(false)
  }, [startCall])

  const endCall = useCallback(async () => {
    return stopCall()
  }, [stopCall])

  // ==================== Utility Methods ====================

  const getUserById = useCallback((id: string) => {
    return users.find(u => u.id === id) || null
  }, [users])

  const getSessionById = useCallback((id: string) => {
    return sessions.find(s => s.id === id) || null
  }, [sessions])

  const getCommentsForFile = useCallback((file: string) => {
    return comments.filter(c => c.file === file)
  }, [comments])

  const getCommentsAtLine = useCallback((file: string, line: number) => {
    return comments.filter(c => c.file === file && c.line === line)
  }, [comments])

  return {
    // State
    connected,
    user,
    users,
    sessions,
    activeSession,
    comments,
    following,
    error,
    isInCall,
    messages,
    audioEnabled,
    videoEnabled,

    // Connection
    connectToServer,
    disconnectFromServer,

    // Session
    createNewSession,
    joinExistingSession,
    leaveCurrentSession,
    shareFile,
    unshareFile,

    // Calls
    startCall,
    stopCall,
    toggleAudio,
    toggleVideo,

    // Comments
    addComment,
    resolveComment,
    replyToComment,

    // Presence
    followUser,
    unfollow,
    updateCursor,
    updateSelection,

    // Video/Audio
    startVideoCall,
    startVoiceCall,
    endCall,

    // Chat
    sendChatMessage,

    // Utilities
    getUserById,
    getSessionById,
    getCommentsForFile,
    getCommentsAtLine,
  }
}