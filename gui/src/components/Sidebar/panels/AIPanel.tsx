import { useState } from 'react'
import { Brain, MessageSquare, Sparkles, Settings } from 'lucide-react'

export default function AIPanel() {
  const [activeTab, setActiveTab] = useState<'copilot' | 'chat' | 'assistant'>('copilot')

  return (
    <div className="ai-panel">
      <div className="ai-tabs">
        <button
          className={activeTab === 'copilot' ? 'active' : ''}
          onClick={() => setActiveTab('copilot')}
        >
          <Sparkles size={14} /> Copilot
        </button>
        <button
          className={activeTab === 'chat' ? 'active' : ''}
          onClick={() => setActiveTab('chat')}
        >
          <MessageSquare size={14} /> Chat
        </button>
        <button
          className={activeTab === 'assistant' ? 'active' : ''}
          onClick={() => setActiveTab('assistant')}
        >
          <Brain size={14} /> Assistant
        </button>
      </div>

      {activeTab === 'copilot' && (
        <div className="copilot-view">
          <div className="copilot-status">
            <span className="status-indicator online" />
            <span>Copilot is active</span>
          </div>
          <div className="copilot-suggestion">
            <h4>Suggestions</h4>
            <div className="suggestion-item">
              <pre>fn main() {`\n    println!("Hello");\n`}</pre>
              <div className="suggestion-actions">
                <button>Accept</button>
                <button>Next</button>
              </div>
            </div>
          </div>
          <div className="copilot-history">
            <h4>Recent</h4>
            <div className="history-item">
              <span>Added error handling</span>
              <span className="time">2m ago</span>
            </div>
          </div>
        </div>
      )}

      {activeTab === 'chat' && (
        <div className="chat-view">
          <div className="chat-messages">
            <div className="message user">
              <div className="message-content">How do I create a Rust project?</div>
            </div>
            <div className="message assistant">
              <div className="message-content">
                Use `cargo new project_name` to create a new Rust project.
                Then `cd project_name` and `cargo run` to build and run.
              </div>
            </div>
          </div>
          <div className="chat-input">
            <input type="text" placeholder="Ask something..." />
            <button>Send</button>
          </div>
        </div>
      )}

      {activeTab === 'assistant' && (
        <div className="assistant-view">
          <div className="assistant-actions">
            <button className="action-btn">Generate Code</button>
            <button className="action-btn">Explain Code</button>
            <button className="action-btn">Refactor</button>
            <button className="action-btn">Find Bugs</button>
          </div>
          <textarea
            placeholder="Enter your code or request..."
            rows={6}
          />
          <div className="assistant-output">
            <h4>Output</h4>
            <pre>// AI output will appear here</pre>
          </div>
        </div>
      )}
    </div>
  )
}