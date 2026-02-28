import { useState, useEffect } from 'react'
import { useDesign } from '../../hooks/useDesign'
import { Search, Grid, List, Download, Copy, Check, Star, Filter } from 'lucide-react'

export default function IconBrowser() {
  const { 
    icons, 
    iconCategories,
    searchIcons,
    getIcon 
  } = useDesign()

  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState('all')
  const [selectedIcon, setSelectedIcon] = useState<any>(null)
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid')
  const [iconSize, setIconSize] = useState<number>(32)
  const [copied, setCopied] = useState(false)
  const [filteredIcons, setFilteredIcons] = useState<any[]>([])

  useEffect(() => {
    loadIcons()
  }, [searchQuery, selectedCategory])

  const loadIcons = async () => {
    if (searchQuery) {
      const results = await searchIcons(searchQuery, selectedCategory !== 'all' ? selectedCategory : undefined)
      setFilteredIcons(results)
    } else {
      setFilteredIcons(icons.slice(0, 100))
    }
  }

  const handleSelectIcon = async (id: string) => {
    const icon = await getIcon(id)
    setSelectedIcon(icon)
  }

  const handleCopySvg = () => {
    if (!selectedIcon) return
    navigator.clipboard.writeText(selectedIcon.svg)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleDownloadSvg = () => {
    if (!selectedIcon) return
    
    const blob = new Blob([selectedIcon.svg], { type: 'image/svg+xml' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${selectedIcon.name}.svg`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleDownloadPng = async () => {
    if (!selectedIcon) return
    
    // This would need to convert SVG to PNG
    // For now, download as SVG
    handleDownloadSvg()
  }

  return (
    <div className="icon-browser">
      <div className="icon-browser-header">
        <h2>Icon Browser</h2>
        <div className="view-controls">
          <button 
            className={viewMode === 'grid' ? 'active' : ''} 
            onClick={() => setViewMode('grid')}
          >
            <Grid size={16} />
          </button>
          <button 
            className={viewMode === 'list' ? 'active' : ''} 
            onClick={() => setViewMode('list')}
          >
            <List size={16} />
          </button>
          <select value={iconSize} onChange={(e) => setIconSize(Number(e.target.value))}>
            <option value={16}>16px</option>
            <option value={24}>24px</option>
            <option value={32}>32px</option>
            <option value={48}>48px</option>
            <option value={64}>64px</option>
          </select>
        </div>
      </div>

      <div className="icon-browser-toolbar">
        <div className="search-bar">
          <Search size={16} />
          <input
            type="text"
            placeholder="Search icons..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <div className="filter-bar">
          <Filter size={16} />
          <select value={selectedCategory} onChange={(e) => setSelectedCategory(e.target.value)}>
            <option value="all">All Categories</option>
            {iconCategories.map(cat => (
              <option key={cat} value={cat}>{cat}</option>
            ))}
          </select>
        </div>
      </div>

      <div className="icon-browser-main">
        <div className={`icon-grid ${viewMode}`}>
          {filteredIcons.map(icon => (
            <div
              key={icon.id}
              className={`icon-item ${selectedIcon?.id === icon.id ? 'selected' : ''}`}
              onClick={() => handleSelectIcon(icon.id)}
            >
              <div 
                className="icon-preview"
                dangerouslySetInnerHTML={{ __html: icon.svg }}
                style={{ width: iconSize, height: iconSize }}
              />
              <div className="icon-name">{icon.name}</div>
              <div className="icon-set">{icon.set}</div>
            </div>
          ))}
        </div>

        {selectedIcon && (
          <div className="icon-detail-panel">
            <h3>{selectedIcon.name}</h3>
            <div className="icon-meta">
              <span>Set: {selectedIcon.set}</span>
              <span>Category: {selectedIcon.category}</span>
              <span>Size: {selectedIcon.width}x{selectedIcon.height}</span>
            </div>

            <div className="icon-preview-large">
              <div dangerouslySetInnerHTML={{ __html: selectedIcon.svg }} />
            </div>

            <div className="icon-actions">
              <button onClick={handleCopySvg}>
                {copied ? <Check size={16} /> : <Copy size={16} />} Copy SVG
              </button>
              <button onClick={handleDownloadSvg}>
                <Download size={16} /> SVG
              </button>
              <button onClick={handleDownloadPng}>
                <Download size={16} /> PNG
              </button>
              <button>
                <Star size={16} /> Favorite
              </button>
            </div>

            <div className="icon-tags">
              {selectedIcon.tags.map((tag: string) => (
                <span key={tag} className="tag">#{tag}</span>
              ))}
            </div>

            <pre className="icon-svg-preview">
              {selectedIcon.svg.substring(0, 200)}...
            </pre>
          </div>
        )}
      </div>
    </div>
  )
}