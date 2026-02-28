import { useState } from 'react'
import { useCollaboration } from '../../hooks/useCollaboration'
import { GitPullRequest, CheckCircle, XCircle, MessageSquare, Clock } from 'lucide-react'

export default function Review() {
  const { 
    activeSession,
    users 
  } = useCollaboration()

  const [reviews, setReviews] = useState<any[]>([
    {
      id: '1',
      title: 'Add authentication feature',
      author: { name: 'Alice' },
      status: 'pending',
      createdAt: Date.now() - 3600000,
      comments: 3,
      files: ['src/auth.rs', 'src/main.rs']
    },
    {
      id: '2',
      title: 'Fix database connection leak',
      author: { name: 'Bob' },
      status: 'approved',
      createdAt: Date.now() - 86400000,
      comments: 5,
      files: ['src/db.rs']
    }
  ])

  const [selectedReview, setSelectedReview] = useState<string | null>(null)

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'approved': return '#6a9955'
      case 'rejected': return '#f48771'
      case 'pending': return '#cca700'
      default: return '#888'
    }
  }

  return (
    <div className="code-review">
      <div className="review-header">
        <h3>
          <GitPullRequest size={16} /> Code Reviews
        </h3>
        <button>New Review</button>
      </div>

      <div className="reviews-list">
        {reviews.map(review => (
          <div
            key={review.id}
            className={`review-item ${selectedReview === review.id ? 'selected' : ''}`}
            onClick={() => setSelectedReview(review.id)}
          >
            <div className="review-title">{review.title}</div>
            <div className="review-meta">
              <span className="review-author">{review.author.name}</span>
              <span className="review-status" style={{ color: getStatusColor(review.status) }}>
                {review.status}
              </span>
            </div>
            <div className="review-stats">
              <span className="review-time">
                <Clock size={12} /> {Math.round((Date.now() - review.createdAt) / 3600000)}h ago
              </span>
              <span className="review-comments">
                <MessageSquare size={12} /> {review.comments}
              </span>
            </div>
          </div>
        ))}
      </div>

      {selectedReview && (
        <div className="review-detail">
          <div className="detail-header">
            <h4>{reviews.find(r => r.id === selectedReview)?.title}</h4>
            <div className="detail-actions">
              <button className="approve">
                <CheckCircle size={16} /> Approve
              </button>
              <button className="reject">
                <XCircle size={16} /> Request Changes
              </button>
            </div>
          </div>

          <div className="detail-files">
            <h5>Files Changed</h5>
            {reviews.find(r => r.id === selectedReview)?.files.map((file: string) => (
              <div key={file} className="file-item">
                <span className="file-name">{file}</span>
                <button>View Diff</button>
              </div>
            ))}
          </div>

          <div className="detail-comments">
            <h5>Comments</h5>
            {/* Comments would go here */}
          </div>
        </div>
      )}
    </div>
  )
}