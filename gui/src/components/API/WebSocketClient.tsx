import { useState, useEffect, useRef } from 'react'
import { useAPI } from '../../hooks/useAPI'
import { 
  Play, Square, Save, Download, Upload, Plus, X,
  Send, Copy, Check, Settings, Wifi, WifiOff
} from 'lucide-react'

export default function WebSocketClient() {
  const { 
    wsConnections,
    connectWebSocket,
    disconnectWebSocket,
    sendWebSocketMessage,
    saveWebSocketRequest,
    loadWebSocketRequest
  } = useAPI()

  const [url, setUrl] = useState('ws://localhost:8080')
  const [messages, setMessages] = useState<Array<{
    type: 'sent' | 'received'
    content: string
    timestamp: number
  }>>([])
  const [connected, setConnected] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [messageInput, setMessageInput] = useState('')
  const [protocols, setProtocols] = useState('')
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>([])
  const [autoReconnect, setAutoReconnect] = useState(true)
  const [showSettings, setShowSettings] = useState(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null)

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const handleConnect = async () => {
    setConnecting(true)
    try {
      await connectWebSocket({
        url,
        protocols: protocols.split(',').map(p => p.trim()).filter(p => p),
        headers: headers.reduce((acc, h) => ({ ...acc, [h.key]: h.value }), {})
      })
      setConnected(true)
    } catch (error) {
      alert('Connection failed: ' + error)
    } finally {
      setConnecting(false)
    }
  }

  const handleDisconnect = async () => {
    await disconnectWebSocket()
    setConnected(false)
  }

  const handleSend = async () => {
    if (!messageInput.trim()) return

    const message = {
      type: 'sent' as const,
      content: messageInput,
      timestamp: Date.now()
    }

    setMessages(prev => [...prev, message])
    await sendWebSocketMessage(messageInput)
    setMessageInput('')
  }

  const handleClear = () => {
    setMessages([])
  }

  const addHeader = () => {
    setHeaders([...headers, { key: '', value: '' }])
  }

  const removeHeader = (index: number) => {
    setHeaders(headers.filter((_, i) => i !== index))
  }

  const copyMessage = (content: string, index: number) => {
    navigator.clipboard.writeText(content)
    setCopiedIndex(index)
    setTimeout(() => setCopiedIndex(null), 2000)
  }

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString()
  }

  return (
    <div className="api-client websocket">
      <div className="connection-bar">
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="WebSocket URL (ws:// or wss://)"
          disabled={connected || connecting}
        />
        {connected ? (
          <button onClick={handleDisconnect} className="danger">
            <Square size={16} /> Disconnect
          </button>
        ) : (
          <button onClick={handleConnect} disabled={connecting}>
            <Play size={16} /> {connecting ? 'Connecting...' : 'Connect'}
          </button>
        )}
        <button onClick={() => setShowSettings(!showSettings)}>
          <Settings size={16} />
        </button>
      </div>

      {showSettings && (
        <div className="connection-settings">
          <div className="settings-row">
            <label>Protocols (comma separated)</label>
            <input
              type="text"
              value={protocols}
              onChange={(e) => setProtocols(e.target.value)}
              placeholder="e.g., json, soap"
            />
          </div>

          <div className="settings-row">
            <label>Headers</label>
            {headers.map((header, i) => (
              <div key={i} className="header-row">
                <input
                  type="text"
                  value={header.key}
                  onChange={(e) => {
                    const newHeaders = [...headers]
                    newHeaders[i].key = e.target.value
                    setHeaders(newHeaders)
                  }}
                  placeholder="Key"
                />
                <input
                  type="text"
                  value={header.value}
                  onChange={(e) => {
                    const newHeaders = [...headers]
                    newHeaders[i].value = e.target.value
                    setHeaders(newHeaders)
                  }}
                  placeholder="Value"
                />
                <button onClick={() => removeHeader(i)}>
                  <X size={14} />
                </button>
              </div>
            ))}
            <button onClick={addHeader} className="add-btn">
              <Plus size={14} /> Add Header
            </button>
          </div>

          <div className="settings-row">
            <label>
              <input
                type="checkbox"
                checked={autoReconnect}
                onChange={(e) => setAutoReconnect(e.target.checked)}
              />
              Auto-reconnect on disconnect
            </label>
          </div>
        </div>
      )}

      <div className="connection-status">
        {connected ? (
          <span className="status connected">
            <Wifi size={14} /> Connected
          </span>
        ) : (
          <span className="status disconnected">
            <WifiOff size={14} /> Disconnected
          </span>
        )}
        <span className="message-count">{messages.length} messages</span>
      </div>

      <div className="messages-container">
        {messages.map((msg, i) => (
          <div
            key={i}
            className={`message ${msg.type}`}
          >
            <div className="message-header">
              <span className="message-time">{formatTimestamp(msg.timestamp)}</span>
              <span className="message-type">{msg.type === 'sent' ? '→' : '←'}</span>
            </div>
            <pre className="message-content">{msg.content}</pre>
            <button
              className="copy-btn"
              onClick={() => copyMessage(msg.content, i)}
            >
              {copiedIndex === i ? <Check size={12} /> : <Copy size={12} />}
            </button>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      <div className="message-input">
        <input
          type="text"
          value={messageInput}
          onChange={(e) => setMessageInput(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSend()}
          placeholder="Type a message..."
          disabled={!connected}
        />
        <button onClick={handleSend} disabled={!connected}>
          <Send size={16} /> Send
        </button>
        <button onClick={handleClear} disabled={messages.length === 0}>
          Clear
        </button>
      </div>
    </div>
  )
}