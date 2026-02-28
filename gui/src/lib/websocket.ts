import ReconnectingWebSocket from 'reconnecting-websocket'

type MessageHandler = (data: any) => void

export class WebSocketClient {
  private ws: ReconnectingWebSocket | null = null
  private handlers: Map<string, Set<MessageHandler>> = new Map()
  private messageQueue: any[] = []
  private connected = false

  constructor(private url: string) {}

  connect(): void {
    if (this.ws) return

    this.ws = new ReconnectingWebSocket(this.url, [], {
      maxReconnectionDelay: 10000,
      minReconnectionDelay: 1000,
      reconnectionDelayGrowFactor: 1.3,
      connectionTimeout: 10000,
      maxRetries: Infinity,
      debug: false
    })

    this.ws.onopen = () => {
      this.connected = true
      this.flushQueue()
      this.emit('connected', {})
    }

    this.ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data)
        const { type, data } = message
        this.handleMessage(type, data)
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error)
      }
    }

    this.ws.onclose = () => {
      this.connected = false
      this.emit('disconnected', {})
    }

    this.ws.onerror = (error) => {
      this.emit('error', { error })
    }
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close()
      this.ws = null
      this.connected = false
    }
  }

  send(type: string, data: any): void {
    const message = JSON.stringify({ type, data })
    
    if (this.connected && this.ws) {
      this.ws.send(message)
    } else {
      this.messageQueue.push(message)
    }
  }

  on(type: string, handler: MessageHandler): () => void {
    if (!this.handlers.has(type)) {
      this.handlers.set(type, new Set())
    }
    this.handlers.get(type)!.add(handler)

    return () => {
      const handlers = this.handlers.get(type)
      if (handlers) {
        handlers.delete(handler)
      }
    }
  }

  off(type: string, handler: MessageHandler): void {
    const handlers = this.handlers.get(type)
    if (handlers) {
      handlers.delete(handler)
    }
  }

  private handleMessage(type: string, data: any): void {
    const handlers = this.handlers.get(type)
    if (handlers) {
      handlers.forEach(handler => handler(data))
    }
  }

  private flushQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift()
      if (this.ws) {
        this.ws.send(message)
      }
    }
  }

  private emit(type: string, data: any): void {
    this.handleMessage(type, data)
  }

  isConnected(): boolean {
    return this.connected
  }
}

// WebSocket message types
export interface WSCursorUpdate {
  userId: string
  file: string
  line: number
  column: number
}

export interface WSSelectionUpdate {
  userId: string
  file: string
  startLine: number
  startCol: number
  endLine: number
  endCol: number
}

export interface WSFileChange {
  userId: string
  file: string
  changes: any[]
}

export interface WSChatMessage {
  userId: string
  userName: string
  message: string
  timestamp: number
}

export interface WSPresenceUpdate {
  userId: string
  userName: string
  status: 'online' | 'away' | 'busy'
  currentFile?: string
}