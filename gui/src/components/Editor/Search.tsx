import { useState, useEffect } from 'react'
import { X, ChevronUp, ChevronDown, Search as SearchIcon } from 'lucide-react'

interface SearchProps {
  onSearch: (query: string, options: SearchOptions) => void
  onClose: () => void
  totalMatches?: number
  currentMatch?: number
}

interface SearchOptions {
  caseSensitive: boolean
  wholeWord: boolean
  regex: boolean
}

export default function Search({ onSearch, onClose, totalMatches = 0, currentMatch = 0 }: SearchProps) {
  const [query, setQuery] = useState('')
  const [replace, setReplace] = useState('')
  const [showReplace, setShowReplace] = useState(false)
  const [options, setOptions] = useState<SearchOptions>({
    caseSensitive: false,
    wholeWord: false,
    regex: false
  })

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      }
      if (e.key === 'Enter') {
        if (e.shiftKey) {
          // Previous match
        } else {
          // Next match
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  const handleSearch = () => {
    if (query.trim()) {
      onSearch(query, options)
    }
  }

  return (
    <div className="search-panel">
      <div className="search-header">
        <div className="search-input-group">
          <SearchIcon size={16} className="search-icon" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="Search"
            autoFocus
          />
          {totalMatches > 0 && (
            <span className="match-count">
              {currentMatch} of {totalMatches}
            </span>
          )}
        </div>
        <div className="search-actions">
          <button onClick={() => setShowReplace(!showReplace)} title="Toggle replace">
            {showReplace ? 'Hide' : 'Show'} Replace
          </button>
          <button onClick={() => setOptions({ ...options, caseSensitive: !options.caseSensitive })}>
            Aa
          </button>
          <button onClick={() => setOptions({ ...options, wholeWord: !options.wholeWord })}>
            \b
          </button>
          <button onClick={() => setOptions({ ...options, regex: !options.regex })}>
            .*
          </button>
          <button onClick={handleSearch} title="Find next">
            <ChevronDown size={16} />
          </button>
          <button onClick={onClose} title="Close">
            <X size={16} />
          </button>
        </div>
      </div>

      {showReplace && (
        <div className="search-replace">
          <input
            type="text"
            value={replace}
            onChange={(e) => setReplace(e.target.value)}
            placeholder="Replace with"
          />
          <div className="replace-actions">
            <button onClick={() => {}}>Replace</button>
            <button onClick={() => {}}>Replace All</button>
          </div>
        </div>
      )}
    </div>
  )
}