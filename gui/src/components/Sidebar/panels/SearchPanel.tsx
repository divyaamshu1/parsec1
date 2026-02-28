import { useState } from 'react'
import { Search, X, File, ChevronRight, ChevronDown } from 'lucide-react'

export default function SearchPanel() {
  const [query, setQuery] = useState('')
  const [replace, setReplace] = useState('')
  const [results, setResults] = useState<any[]>([])
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [options, setOptions] = useState({
    matchCase: false,
    wholeWord: false,
    regex: false,
    include: '',
    exclude: '**/node_modules/**'
  })

  const handleSearch = () => {
    // Mock search results
    setResults([
      {
        file: 'src/main.rs',
        matches: [
          { line: 10, content: 'fn main() {', column: 0 },
          { line: 15, content: '    println!("Hello");', column: 4 },
        ]
      },
      {
        file: 'src/lib.rs',
        matches: [
          { line: 5, content: 'pub fn test() {', column: 0 },
        ]
      }
    ])
  }

  const toggleExpand = (file: string) => {
    const newExpanded = new Set(expanded)
    if (expanded.has(file)) {
      newExpanded.delete(file)
    } else {
      newExpanded.add(file)
    }
    setExpanded(newExpanded)
  }

  const handleReplace = () => {
    // Replace functionality
  }

  const handleReplaceAll = () => {
    // Replace all functionality
  }

  return (
    <div className="search-panel">
      <div className="search-inputs">
        <div className="search-field">
          <Search size={14} />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search"
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          />
          {query && (
            <button onClick={() => setQuery('')}>
              <X size={12} />
            </button>
          )}
        </div>
        <div className="search-field">
          <input
            type="text"
            value={replace}
            onChange={(e) => setReplace(e.target.value)}
            placeholder="Replace with"
          />
        </div>
      </div>

      <div className="search-options">
        <label>
          <input
            type="checkbox"
            checked={options.matchCase}
            onChange={(e) => setOptions({ ...options, matchCase: e.target.checked })}
          />
          Match Case
        </label>
        <label>
          <input
            type="checkbox"
            checked={options.wholeWord}
            onChange={(e) => setOptions({ ...options, wholeWord: e.target.checked })}
          />
          Whole Word
        </label>
        <label>
          <input
            type="checkbox"
            checked={options.regex}
            onChange={(e) => setOptions({ ...options, regex: e.target.checked })}
          />
          Regular Expression
        </label>
      </div>

      <div className="search-actions">
        <button onClick={handleSearch}>Search</button>
        <button onClick={handleReplace}>Replace</button>
        <button onClick={handleReplaceAll}>Replace All</button>
      </div>

      {results.length > 0 && (
        <div className="search-results">
          <div className="results-summary">
            {results.length} files • {results.reduce((acc, r) => acc + r.matches.length, 0)} matches
          </div>

          {results.map(result => {
            const isExpanded = expanded.has(result.file)
            return (
              <div key={result.file} className="result-file">
                <div
                  className="result-file-header"
                  onClick={() => toggleExpand(result.file)}
                >
                  {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  <File size={14} />
                  <span className="file-name">{result.file}</span>
                  <span className="match-count">{result.matches.length}</span>
                </div>

                {isExpanded && (
                  <div className="result-matches">
                    {result.matches.map((match: any, i: number) => (
                      <div key={i} className="result-match">
                        <div className="match-line">
                          <span className="line-number">{match.line}</span>
                          <span className="line-content">
                            {match.content.substring(0, match.column)}
                            <mark>{match.content.substring(match.column, match.column + query.length)}</mark>
                            {match.content.substring(match.column + query.length)}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}