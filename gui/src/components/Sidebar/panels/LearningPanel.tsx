import { useState } from 'react'
import { 
  BookOpen, Play, Code, Award, Clock,
  ChevronRight, Star, Filter
} from 'lucide-react'

export default function LearningPanel() {
  const [tutorials] = useState([
    { id: '1', title: 'Rust Basics', level: 'beginner', duration: 30, completed: true },
    { id: '2', title: 'WebAssembly', level: 'intermediate', duration: 45, completed: false },
    { id: '3', title: 'Async Rust', level: 'advanced', duration: 60, completed: false },
  ])

  const [snippets] = useState([
    { id: '1', title: 'HTTP Request', language: 'rust', stars: 42 },
    { id: '2', title: 'Database Query', language: 'sql', stars: 28 },
    { id: '3', title: 'React Hook', language: 'typescript', stars: 35 },
  ])

  return (
    <div className="learning-panel">
      <div className="panel-actions">
        <button className="primary">
          <Play size={14} /> Start Learning
        </button>
        <button>
          <Filter size={14} />
        </button>
      </div>

      <div className="tutorials-section">
        <h4>Continue Learning</h4>
        {tutorials.map(tutorial => (
          <div key={tutorial.id} className="tutorial-item">
            <BookOpen size={16} />
            <div className="tutorial-info">
              <div className="tutorial-title">{tutorial.title}</div>
              <div className="tutorial-meta">
                <span className={`level ${tutorial.level}`}>{tutorial.level}</span>
                <span className="duration">
                  <Clock size={12} /> {tutorial.duration}min
                </span>
              </div>
            </div>
            {tutorial.completed && <Award size={14} className="completed" />}
            <ChevronRight size={14} />
          </div>
        ))}
      </div>

      <div className="snippets-section">
        <h4>Popular Snippets</h4>
        {snippets.map(snippet => (
          <div key={snippet.id} className="snippet-item">
            <Code size={16} />
            <div className="snippet-info">
              <div className="snippet-title">{snippet.title}</div>
              <div className="snippet-meta">
                <span className="language">{snippet.language}</span>
                <span className="stars">
                  <Star size={12} /> {snippet.stars}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="quick-actions">
        <button className="action-btn">
          <Code size={14} /> Code Playground
        </button>
        <button className="action-btn">
          <BookOpen size={14} /> Cheat Sheets
        </button>
      </div>
    </div>
  )
}