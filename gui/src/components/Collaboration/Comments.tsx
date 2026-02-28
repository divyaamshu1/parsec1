import { useState } from 'react'
import { useCollaboration } from '../../hooks/useCollaboration'
import { MessageSquare, CheckCircle, XCircle, CornerDownRight } from 'lucide-react'

export default function Comments() {
  const { 
    comments, 
    addComment, 
    resolveComment, 
    replyToComment 
  } = useCollaboration()

  const [newComment, setNewComment] = useState('')
  const [selectedLine, setSelectedLine] = useState<number | null>(null)
  const [replyingTo, setReplyingTo] = useState<string | null>(null)
  const [replyText, setReplyText] = useState('')

  const handleAddComment = () => {
    if (!newComment.trim() || selectedLine === null) return
    
    addComment('current-file', selectedLine, newComment)
    setNewComment('')
    setSelectedLine(null)
  }

  const handleReply = (commentId: string) => {
    if (!replyText.trim()) return
    
    replyToComment(commentId, replyText)
    setReplyText('')
    setReplyingTo(null)
  }

  const getCommentsForLine = (line: number) => {
    return comments.filter(c => c.line === line)
  }

  return (
    <div className="comments-panel">
      <div className="comments-header">
        <h3>
          <MessageSquare size={16} /> Comments
        </h3>
      </div>

      <div className="new-comment">
        <div className="comment-line-selector">
          <input
            type="number"
            placeholder="Line"
            value={selectedLine || ''}
            onChange={(e) => setSelectedLine(parseInt(e.target.value) || null)}
          />
        </div>
        <textarea
          value={newComment}
          onChange={(e) => setNewComment(e.target.value)}
          placeholder="Write a comment..."
          rows={3}
        />
        <button onClick={handleAddComment}>Add Comment</button>
      </div>

      <div className="comments-list">
        {comments.map(comment => (
          <div key={comment.id} className="comment">
            <div className="comment-header">
              <div className="comment-author">
                <span className="author-name">{comment.author.name}</span>
                <span className="comment-line">Line {comment.line}</span>
              </div>
              <div className="comment-actions">
                {!comment.resolved && (
                  <button onClick={() => resolveComment(comment.id)}>
                    <CheckCircle size={14} />
                  </button>
                )}
                <button onClick={() => setReplyingTo(comment.id)}>
                  <CornerDownRight size={14} />
                </button>
              </div>
            </div>

            <div className="comment-content">
              {comment.text}
            </div>

            <div className="comment-time">
              {new Date(comment.createdAt).toLocaleString()}
            </div>

            {comment.resolved && (
              <div className="comment-resolved">
                ✓ Resolved
              </div>
            )}

            {comment.replies && comment.replies.length > 0 && (
              <div className="comment-replies">
                {comment.replies.map(reply => (
                  <div key={reply.id} className="reply">
                    <div className="reply-author">{reply.author.name}</div>
                    <div className="reply-content">{reply.text}</div>
                  </div>
                ))}
              </div>
            )}

            {replyingTo === comment.id && (
              <div className="reply-input">
                <textarea
                  value={replyText}
                  onChange={(e) => setReplyText(e.target.value)}
                  placeholder="Write a reply..."
                  rows={2}
                />
                <div className="reply-actions">
                  <button onClick={() => handleReply(comment.id)}>Reply</button>
                  <button onClick={() => setReplyingTo(null)}>Cancel</button>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}