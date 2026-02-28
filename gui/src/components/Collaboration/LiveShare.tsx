import { useState, useEffect } from 'react'
import { useCollaboration } from '../../hooks/useCollaboration'
import { Users, Video, Mic, MicOff, VideoOff, Share2, UserPlus, LogOut } from 'lucide-react'

export default function LiveShare() {
  const { 
    connected,
    user,
    users,
    activeSession,
    isInCall,
    connectToServer,
    disconnectFromServer,
    createNewSession,
    joinExistingSession,
    leaveCurrentSession,
    startCall,
    stopCall,
    sendChatMessage
  } = useCollaboration()

  const [server, setServer] = useState('wss://signaling.parsec.dev')
  const [sessionName, setSessionName] = useState('')
  const [sessionPassword, setSessionPassword] = useState('')
  const [message, setMessage] = useState('')
  const [messages, setMessages] = useState<any[]>([])
  const [audioEnabled, setAudioEnabled] = useState(true)
  const [videoEnabled, setVideoEnabled] = useState(false)

  useEffect(() => {
    if (connected && activeSession) {
      // Subscribe to messages
    }
  }, [connected, activeSession])

  const handleConnect = async () => {
    await connectToServer(server)
  }

  const handleCreateSession = async () => {
    await createNewSession(sessionName, sessionPassword)
  }

  const handleJoinSession = async (id: string) => {
    await joinExistingSession(id, sessionPassword)
  }

  const handleSendMessage = () => {
    if (!message.trim()) return
    sendChatMessage(message)
    setMessages([...messages, { from: user?.name, text: message, timestamp: Date.now() }])
    setMessage('')
  }

  const toggleAudio = () => {
    setAudioEnabled(!audioEnabled)
    // Would update audio track
  }

  const toggleVideo = () => {
    setVideoEnabled(!videoEnabled)
    // Would update video track
  }

  if (!connected) {
    return (
      <div className="live-share">
        <div className="connect-panel">
          <h3>Connect to Collaboration Server</h3>
          <input
            type="text"
            value={server}
            onChange={(e) => setServer(e.target.value)}
            placeholder="Server URL"
          />
          <button onClick={handleConnect}>Connect</button>
        </div>
      </div>
    )
  }

  if (!activeSession) {
    return (
      <div className="live-share">
        <div className="user-info">
          <span className="user-name">{user?.name}</span>
          <span className="connection-status">Connected</span>
        </div>

        <div className="sessions-panel">
          <h3>Sessions</h3>
          <div className="create-session">
            <input
              type="text"
              value={sessionName}
              onChange={(e) => setSessionName(e.target.value)}
              placeholder="Session Name"
            />
            <input
              type="password"
              value={sessionPassword}
              onChange={(e) => setSessionPassword(e.target.value)}
              placeholder="Password (optional)"
            />
            <button onClick={handleCreateSession}>
              <UserPlus size={16} /> Create Session
            </button>
          </div>

          <div className="sessions-list">
            {/* Session list would be populated here */}
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="live-share active">
      <div className="call-header">
        <div className="session-info">
          <h3>{activeSession.name}</h3>
          <span className="participant-count">
            <Users size={14} /> {activeSession.participants.length}
          </span>
        </div>
        <div className="call-controls">
          <button onClick={toggleAudio} className={audioEnabled ? '' : 'muted'}>
            {audioEnabled ? <Mic size={18} /> : <MicOff size={18} />}
          </button>
          <button onClick={toggleVideo} className={videoEnabled ? '' : 'muted'}>
            {videoEnabled ? <Video size={18} /> : <VideoOff size={18} />}
          </button>
          {isInCall ? (
            <button onClick={stopCall} className="danger">
              End Call
            </button>
          ) : (
            <button onClick={() => startCall(false)}>
              <Share2 size={16} /> Start Call
            </button>
          )}
          <button onClick={leaveCurrentSession} className="danger">
            <LogOut size={16} /> Leave
          </button>
        </div>
      </div>

      <div className="participants-grid">
        {activeSession.participants.map(p => (
          <div key={p.id} className="participant">
            <div className="participant-avatar">
              {p.avatar ? (
                <img src={p.avatar} alt={p.name} />
              ) : (
                <div className="avatar-placeholder">
                  {p.name.charAt(0).toUpperCase()}
                </div>
              )}
            </div>
            <div className="participant-name">{p.name}</div>
            {p.cursor && (
              <div className="participant-cursor">
                {p.cursor.line}:{p.cursor.column}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="chat-panel">
        <div className="chat-messages">
          {messages.map((msg, i) => (
            <div key={i} className="message">
              <span className="message-author">{msg.from}:</span>
              <span className="message-text">{msg.text}</span>
              <span className="message-time">
                {new Date(msg.timestamp).toLocaleTimeString()}
              </span>
            </div>
          ))}
        </div>
        <div className="chat-input">
          <input
            type="text"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSendMessage()}
            placeholder="Type a message..."
          />
          <button onClick={handleSendMessage}>Send</button>
        </div>
      </div>
    </div>
  )
}