import { useState, useRef, useEffect } from 'react'
import { useAI } from '../../hooks/useAI'
import { 
  Send, User, Bot, Loader, Copy, Check,
  ThumbsUp, ThumbsDown, RefreshCw, Trash2
} from 'lucide-react'

export default function AIChat() {
  const { 
    chatMessages,
    isGenerating,
    sendChatMessage,
    streamChatMessage,
    clearChat
  } = useAI()

  const [input, setInput] = useState('')
  const [streamingMessage, setStreamingMessage] = useState('')
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    scrollToBottom()
  }, [chatMessages, streamingMessage])

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const handleSend = async () => {
    if (!input.trim() || isGenerating) return

    const userMessage = input
    setInput('')
    
    if (streamingMessage) {
      setStreamingMessage('')
    }

    try {
      await streamChatMessage(userMessage, (token) => {
        setStreamingMessage(prev => prev + token)
      })
    } catch (error) {
      console.error('Chat error:', error)
    } finally {
      setStreamingMessage('')
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const copyMessage = (content: string, index: number) => {
    navigator.clipboard.writeText(content)
    setCopiedIndex(index)
    setTimeout(() => setCopiedIndex(null), 2000)
  }

  const handleClear = () => {
    if (confirm('Clear all chat messages?')) {
      clearChat()
    }
  }

  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }

  return (
    <div className="ai-panel chat">
      <div className="panel-header">
        <h3>
          <Bot size={18} /> AI Chat
        </h3>
        <button onClick={handleClear} disabled={chatMessages.length === 0}>
          <Trash2 size={16} />
        </button>
      </div>

      <div className="messages-container">
        {chatMessages.map((msg, index) => (
          <div key={index} className={`message ${msg.role}`}>
            <div className="message-avatar">
              {msg.role === 'user' ? <User size={16} /> : <Bot size={16} />}
            </div>
            <div className="message-content">
              <div className="message-header">
                <span className="message-role">
                  {msg.role === 'user' ? 'You' : 'AI Assistant'}
                </span>
                <span className="message-time">{formatTime(msg.timestamp)}</span>
              </div>
              <pre className="message-text">{msg.content}</pre>
              <div className="message-actions">
                <button onClick={() => copyMessage(msg.content, index)}>
                  {copiedIndex === index ? <Check size={12} /> : <Copy size={12} />}
                </button>
                <button>
                  <ThumbsUp size={12} />
                </button>
                <button>
                  <ThumbsDown size={12} />
                </button>
              </div>
            </div>
          </div>
        ))}

        {streamingMessage && (
          <div className="message assistant streaming">
            <div className="message-avatar">
              <Bot size={16} />
            </div>
            <div className="message-content">
              <div className="message-header">
                <span className="message-role">AI Assistant</span>
                <Loader size={12} className="spin" />
              </div>
              <pre className="message-text">{streamingMessage}</pre>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      <div className="input-area">
        <textarea
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Ask me anything..."
          rows={1}
          disabled={isGenerating}
        />
        <button onClick={handleSend} disabled={isGenerating || !input.trim()}>
          <Send size={16} />
        </button>
      </div>

      <div className="chat-footer">
        <span>AI can make mistakes. Verify important information.</span>
      </div>
    </div>
  )
}