import { useState } from 'react'
import { 
  Users, Video, Mic, MicOff, VideoOff,
  Share2, UserPlus, MessageSquare, Phone
} from 'lucide-react'

export default function CollaborationPanel() {
  const [participants] = useState([
    { id: '1', name: 'Alice', status: 'online', audio: true, video: false },
    { id: '2', name: 'Bob', status: 'online', audio: true, video: true },
    { id: '3', name: 'Charlie', status: 'away', audio: false, video: false },
  ])

  const [messages] = useState([
    { user: 'Alice', text: 'Hey team!', time: '10:00' },
    { user: 'Bob', text: 'Ready for review', time: '10:02' },
  ])

  const [inCall, setInCall] = useState(false)

  return (
    <div className="collaboration-panel">
      <div className="session-info">
        <h4>Current Session</h4>
        <div className="session-name">Project Review</div>
        <div className="participant-count">
          <Users size={14} /> {participants.length} participants
        </div>
      </div>

      <div className="call-controls">
        {inCall ? (
          <>
            <button className="call-btn active">
              <Mic size={16} />
            </button>
            <button className="call-btn">
              <VideoOff size={16} />
            </button>
            <button className="call-btn end" onClick={() => setInCall(false)}>
              <Phone size={16} />
            </button>
          </>
        ) : (
          <button className="start-call" onClick={() => setInCall(true)}>
            <Video size={16} /> Start Call
          </button>
        )}
      </div>

      <div className="participants-list">
        <h4>Participants</h4>
        {participants.map(p => (
          <div key={p.id} className="participant-item">
            <div className="participant-avatar">
              {p.name.charAt(0)}
            </div>
            <div className="participant-info">
              <div className="participant-name">{p.name}</div>
              <div className="participant-status">{p.status}</div>
            </div>
            <div className="participant-media">
              {p.audio && <Mic size={12} />}
              {p.video && <Video size={12} />}
            </div>
          </div>
        ))}
      </div>

      <div className="chat-section">
        <h4>Chat</h4>
        <div className="chat-messages">
          {messages.map((msg, i) => (
            <div key={i} className="chat-message">
              <span className="message-user">{msg.user}</span>
              <span className="message-text">{msg.text}</span>
              <span className="message-time">{msg.time}</span>
            </div>
          ))}
        </div>
        <div className="chat-input">
          <input type="text" placeholder="Type a message..." />
          <button>
            <MessageSquare size={14} />
          </button>
        </div>
      </div>
    </div>
  )
}