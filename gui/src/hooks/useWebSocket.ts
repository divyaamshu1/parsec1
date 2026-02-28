import { useEffect, useRef, useCallback } from 'react'
import ReconnectingWebSocket from 'reconnecting-websocket'

export function useWebSocket(url: string) {
  const ws = useRef<ReconnectingWebSocket | null>(null)
  const messageHandlers = useRef<Map<string, Set<(data: any) => void>>>(new Map())

  const connect = useCallback(() => {
    if (!ws.current) {
      ws.current = new ReconnectingWebSocket(url)
      
      ws.current.onopen = () => {
        console.log('WebSocket connected')
      }

      ws.current.onmessage = (event) => {
        try {
          const { type, data } = JSON.parse(event.data)
          const handlers = messageHandlers.current.get(type)
          if (handlers) {
            handlers.forEach(handler => handler(data))
          }
        } catch (error) {
          console.error('WebSocket message error:', error)
        }
      }

      ws.current.onerror = (error) => {
        console.error('WebSocket error:', error)
      }

      ws.current.onclose = () => {
        console.log('WebSocket closed')
      }
    }
  }, [url])

  const disconnect = useCallback(() => {
    if (ws.current) {
      ws.current.close()
      ws.current = null
    }
  }, [])

  const send = useCallback((type: string, data: any) => {
    if (ws.current && ws.current.readyState === WebSocket.OPEN) {
      ws.current.send(JSON.stringify({ type, data }))
    }
  }, [])

  const on = useCallback((type: string, handler: (data: any) => void) => {
    if (!messageHandlers.current.has(type)) {
      messageHandlers.current.set(type, new Set())
    }
    messageHandlers.current.get(type)!.add(handler)

    return () => {
      const handlers = messageHandlers.current.get(type)
      if (handlers) {
        handlers.delete(handler)
        if (handlers.size === 0) {
          messageHandlers.current.delete(type)
        }
      }
    }
  }, [])

  const off = useCallback((type: string, handler: (data: any) => void) => {
    const handlers = messageHandlers.current.get(type)
    if (handlers) {
      handlers.delete(handler)
    }
  }, [])

  useEffect(() => {
    connect()
    return () => disconnect()
  }, [connect, disconnect])

  return { send, on, off, isConnected: ws.current?.readyState === WebSocket.OPEN }
}