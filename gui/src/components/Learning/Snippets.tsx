import { useState, useEffect } from 'react'
import { useLearning } from '../../hooks/useLearning'
import { Search, Star, Code, Copy, Check, Filter } from 'lucide-react'
import Editor from '@monaco-editor/react'

export default function Snippets() {
  const { 
    snippets, 
    searchSnippetLibrary,
    starExistingSnippet,
    getSnippetById,
    saveNewSnippet
  } = useLearning()

  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<any[]>([])
  const [selectedSnippet, setSelectedSnippet] = useState<any>(null)
  const [selectedLanguage, setSelectedLanguage] = useState<string>('all')
  const [showNewForm, setShowNewForm] = useState(false)
  const [newSnippet, setNewSnippet] = useState({
    title: '',
    description: '',
    code: '',
    language: 'rust',
    tags: ''
  })
  const [copied, setCopied] = useState(false)

  const languages = ['all', 'rust', 'python', 'javascript', 'typescript', 'go', 'java', 'cpp', 'csharp']

  useEffect(() => {
    if (searchQuery) {
      handleSearch()
    }
  }, [searchQuery, selectedLanguage])

  const handleSearch = async () => {
    const results = await searchSnippetLibrary(searchQuery, selectedLanguage !== 'all' ? selectedLanguage : undefined)
    setSearchResults(results)
  }

  const handleSelectSnippet = async (id: string) => {
    const snippet = await getSnippetById(id)
    setSelectedSnippet(snippet)
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(selectedSnippet.code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleStar = async (id: string) => {
    await starExistingSnippet(id)
    if (selectedSnippet?.id === id) {
      setSelectedSnippet({ ...selectedSnippet, stars: selectedSnippet.stars + 1 })
    }
  }

  const handleSaveSnippet = async () => {
    await saveNewSnippet({
      title: newSnippet.title,
      description: newSnippet.description,
      code: newSnippet.code,
      language: newSnippet.language,
      tags: newSnippet.tags.split(',').map(t => t.trim())
    })
    setShowNewForm(false)
    setNewSnippet({ title: '', description: '', code: '', language: 'rust', tags: '' })
  }

  return (
    <div className="snippets">
      <div className="snippets-sidebar">
        <div className="search-bar">
          <Search size={16} />
          <input
            type="text"
            placeholder="Search snippets..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <div className="filter-bar">
          <Filter size={16} />
          <select value={selectedLanguage} onChange={(e) => setSelectedLanguage(e.target.value)}>
            {languages.map(lang => (
              <option key={lang} value={lang}>
                {lang === 'all' ? 'All Languages' : lang}
              </option>
            ))}
          </select>
        </div>

        <button className="new-snippet-btn" onClick={() => setShowNewForm(true)}>
          + New Snippet
        </button>

        <div className="snippets-list">
          {searchResults.map(snippet => (
            <div
              key={snippet.id}
              className={`snippet-item ${selectedSnippet?.id === snippet.id ? 'active' : ''}`}
              onClick={() => handleSelectSnippet(snippet.id)}
            >
              <div className="snippet-title">{snippet.title}</div>
              <div className="snippet-meta">
                <span className="snippet-language">{snippet.language}</span>
                <span className="snippet-stars">
                  <Star size={12} /> {snippet.stars}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="snippet-view">
        {selectedSnippet ? (
          <>
            <div className="snippet-header">
              <div>
                <h2>{selectedSnippet.title}</h2>
                <p className="snippet-description">{selectedSnippet.description}</p>
                <div className="snippet-tags">
                  {selectedSnippet.tags?.map((tag: string) => (
                    <span key={tag} className="tag">#{tag}</span>
                  ))}
                </div>
              </div>
              <div className="snippet-actions">
                <button onClick={handleCopy}>
                  {copied ? <Check size={16} /> : <Copy size={16} />}
                </button>
                <button onClick={() => handleStar(selectedSnippet.id)}>
                  <Star size={16} /> {selectedSnippet.stars}
                </button>
              </div>
            </div>

            <div className="snippet-code">
              <Editor
                height="400px"
                defaultLanguage={selectedSnippet.language}
                value={selectedSnippet.code}
                theme="vs-dark"
                options={{
                  readOnly: true,
                  fontSize: 13,
                  minimap: { enabled: false },
                  lineNumbers: 'on'
                }}
              />
            </div>
          </>
        ) : (
          <div className="snippet-empty">
            <Code size={48} />
            <h3>Select a snippet</h3>
            <p>Choose a snippet from the list to view its code</p>
          </div>
        )}
      </div>

      {showNewForm && (
        <div className="modal-overlay">
          <div className="modal new-snippet-modal">
            <div className="modal-header">
              <h3>Create New Snippet</h3>
              <button onClick={() => setShowNewForm(false)}>✕</button>
            </div>
            <div className="modal-body">
              <input
                type="text"
                placeholder="Title"
                value={newSnippet.title}
                onChange={(e) => setNewSnippet({ ...newSnippet, title: e.target.value })}
              />
              <textarea
                placeholder="Description"
                value={newSnippet.description}
                onChange={(e) => setNewSnippet({ ...newSnippet, description: e.target.value })}
                rows={2}
              />
              <select
                value={newSnippet.language}
                onChange={(e) => setNewSnippet({ ...newSnippet, language: e.target.value })}
              >
                {languages.filter(l => l !== 'all').map(lang => (
                  <option key={lang} value={lang}>{lang}</option>
                ))}
              </select>
              <textarea
                placeholder="Code"
                value={newSnippet.code}
                onChange={(e) => setNewSnippet({ ...newSnippet, code: e.target.value })}
                rows={10}
                className="code-input"
              />
              <input
                type="text"
                placeholder="Tags (comma separated)"
                value={newSnippet.tags}
                onChange={(e) => setNewSnippet({ ...newSnippet, tags: e.target.value })}
              />
            </div>
            <div className="modal-footer">
              <button onClick={handleSaveSnippet}>Create</button>
              <button onClick={() => setShowNewForm(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}