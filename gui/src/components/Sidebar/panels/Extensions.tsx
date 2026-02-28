import { useState } from 'react'
import { 
  Puzzle, Search, Download, Trash2, 
  Check, X, Settings, Star
} from 'lucide-react'

export default function Extensions() {
  const [search, setSearch] = useState('')
  const [installed, setInstalled] = useState([
    { id: 'rust-lang.rust', name: 'Rust', publisher: 'rust-lang', version: '1.0.0', enabled: true, installed: true },
    { id: 'ms-python.python', name: 'Python', publisher: 'Microsoft', version: '2024.1.0', enabled: true, installed: true },
    { id: 'github.copilot', name: 'GitHub Copilot', publisher: 'GitHub', version: '1.150.0', enabled: true, installed: true },
  ])

  const [marketplace, setMarketplace] = useState([
    { id: 'esbenp.prettier', name: 'Prettier', publisher: 'Prettier', description: 'Code formatter', downloads: '10M', rating: 4.5 },
    { id: 'dbaeumer.vscode-eslint', name: 'ESLint', publisher: 'Microsoft', description: 'JavaScript linter', downloads: '20M', rating: 4.8 },
    { id: 'ms-vscode-remote.remote-ssh', name: 'Remote - SSH', publisher: 'Microsoft', description: 'Remote development', downloads: '5M', rating: 4.3 },
  ])

  const [showInstalled, setShowInstalled] = useState(true)

  const toggleExtension = (id: string) => {
    setInstalled(installed.map(ext =>
      ext.id === id ? { ...ext, enabled: !ext.enabled } : ext
    ))
  }

  const uninstallExtension = (id: string) => {
    setInstalled(installed.filter(ext => ext.id !== id))
  }

  const installExtension = (ext: any) => {
    setInstalled([...installed, { ...ext, enabled: true, installed: true, version: 'latest' }])
  }

  return (
    <div className="extensions-panel">
      <div className="extensions-header">
        <div className="search-bar">
          <Search size={14} />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search extensions"
          />
        </div>
        <div className="view-toggle">
          <button
            className={showInstalled ? 'active' : ''}
            onClick={() => setShowInstalled(true)}
          >
            Installed
          </button>
          <button
            className={!showInstalled ? 'active' : ''}
            onClick={() => setShowInstalled(false)}
          >
            Marketplace
          </button>
        </div>
      </div>

      {showInstalled ? (
        <div className="installed-list">
          {installed.map(ext => (
            <div key={ext.id} className="extension-item">
              <div className="extension-icon">
                <Puzzle size={24} />
              </div>
              <div className="extension-info">
                <div className="extension-name">{ext.name}</div>
                <div className="extension-meta">
                  {ext.publisher} • {ext.version}
                </div>
              </div>
              <div className="extension-actions">
                <button onClick={() => toggleExtension(ext.id)}>
                  {ext.enabled ? <Check size={14} /> : <X size={14} />}
                </button>
                <button onClick={() => uninstallExtension(ext.id)}>
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="marketplace-list">
          {marketplace
            .filter(ext => ext.name.toLowerCase().includes(search.toLowerCase()))
            .map(ext => (
              <div key={ext.id} className="marketplace-item">
                <div className="extension-icon">
                  <Puzzle size={24} />
                </div>
                <div className="extension-info">
                  <div className="extension-name">{ext.name}</div>
                  <div className="extension-publisher">{ext.publisher}</div>
                  <div className="extension-desc">{ext.description}</div>
                  <div className="extension-stats">
                    <span>{ext.downloads} downloads</span>
                    <span className="rating">
                      <Star size={12} /> {ext.rating}
                    </span>
                  </div>
                </div>
                <button onClick={() => installExtension(ext)}>
                  <Download size={14} /> Install
                </button>
              </div>
            ))}
        </div>
      )}
    </div>
  )
}