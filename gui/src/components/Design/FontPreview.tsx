import { useState, useEffect } from 'react'
import { useDesign } from '../../hooks/useDesign'
import { Search, Bold, Italic, Underline, Type, Download, ChevronDown } from 'lucide-react'

export default function FontPreview() {
  const { 
    fonts, 
    loadFonts,
    previewFont,
    loadFont 
  } = useDesign()

  const [searchQuery, setSearchQuery] = useState('')
  const [selectedFont, setSelectedFont] = useState<any>(null)
  const [previewText, setPreviewText] = useState('The quick brown fox jumps over the lazy dog')
  const [fontSize, setFontSize] = useState(48)
  const [fontWeight, setFontWeight] = useState('400')
  const [fontStyle, setFontStyle] = useState('normal')
  const [textDecoration, setTextDecoration] = useState('none')
  const [previewImage, setPreviewImage] = useState<string | null>(null)
  const [filteredFonts, setFilteredFonts] = useState<any[]>([])

  useEffect(() => {
    loadFonts()
  }, [])

  useEffect(() => {
    if (searchQuery) {
      setFilteredFonts(fonts.filter(f => 
        f.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        f.family.toLowerCase().includes(searchQuery.toLowerCase())
      ))
    } else {
      setFilteredFonts(fonts)
    }
  }, [searchQuery, fonts])

  useEffect(() => {
    if (selectedFont) {
      generatePreview()
    }
  }, [selectedFont, previewText, fontSize, fontWeight, fontStyle, textDecoration])

  const generatePreview = async () => {
    if (!selectedFont) return
    
    const img = await previewFont(selectedFont.id, previewText, fontSize)
    setPreviewImage(`data:image/png;base64,${img}`)
  }

  const handleSelectFont = async (id: string) => {
    const font = await loadFont(id)
    setSelectedFont(font)
  }

  const weightOptions = [
    { value: '100', label: 'Thin' },
    { value: '300', label: 'Light' },
    { value: '400', label: 'Regular' },
    { value: '500', label: 'Medium' },
    { value: '700', label: 'Bold' },
    { value: '900', label: 'Black' }
  ]

  return (
    <div className="font-preview">
      <div className="font-preview-header">
        <h2>Font Preview</h2>
      </div>

      <div className="font-preview-main">
        <div className="font-sidebar">
          <div className="search-bar">
            <Search size={16} />
            <input
              type="text"
              placeholder="Search fonts..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>

          <div className="fonts-list">
            {filteredFonts.map(font => (
              <div
                key={font.id}
                className={`font-item ${selectedFont?.id === font.id ? 'selected' : ''}`}
                onClick={() => handleSelectFont(font.id)}
              >
                <div className="font-name">{font.name}</div>
                <div className="font-family">{font.family}</div>
                <div className="font-meta">
                  <span>{font.weight}</span>
                  <span>{font.style}</span>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="preview-area">
          {selectedFont ? (
            <>
              <div className="preview-controls">
                <input
                  type="text"
                  value={previewText}
                  onChange={(e) => setPreviewText(e.target.value)}
                  className="preview-text-input"
                  placeholder="Preview text..."
                />

                <div className="control-group">
                  <label>Size</label>
                  <input
                    type="number"
                    value={fontSize}
                    onChange={(e) => setFontSize(Number(e.target.value))}
                    min="8"
                    max="200"
                  />
                </div>

                <div className="control-group">
                  <label>Weight</label>
                  <select value={fontWeight} onChange={(e) => setFontWeight(e.target.value)}>
                    {weightOptions.map(opt => (
                      <option key={opt.value} value={opt.value}>{opt.label}</option>
                    ))}
                  </select>
                </div>

                <div className="control-group buttons">
                  <button 
                    className={fontStyle === 'italic' ? 'active' : ''}
                    onClick={() => setFontStyle(fontStyle === 'italic' ? 'normal' : 'italic')}
                  >
                    <Italic size={16} />
                  </button>
                  <button 
                    className={textDecoration === 'underline' ? 'active' : ''}
                    onClick={() => setTextDecoration(textDecoration === 'underline' ? 'none' : 'underline')}
                  >
                    <Underline size={16} />
                  </button>
                </div>
              </div>

              <div className="preview-display">
                {previewImage ? (
                  <img src={previewImage} alt="Font Preview" />
                ) : (
                  <div className="preview-text" style={{
                    fontFamily: selectedFont.family,
                    fontSize: `${fontSize}px`,
                    fontWeight,
                    fontStyle,
                    textDecoration
                  }}>
                    {previewText}
                  </div>
                )}
              </div>

              <div className="font-details">
                <h4>Font Details</h4>
                <table>
                  <tr>
                    <td>Family</td>
                    <td>{selectedFont.family}</td>
                  </tr>
                  <tr>
                    <td>Weight</td>
                    <td>{selectedFont.weight}</td>
                  </tr>
                  <tr>
                    <td>Style</td>
                    <td>{selectedFont.style}</td>
                  </tr>
                  <tr>
                    <td>Version</td>
                    <td>{selectedFont.version || '1.0'}</td>
                  </tr>
                  <tr>
                    <td>Glyphs</td>
                    <td>{selectedFont.glyphs || 'N/A'}</td>
                  </tr>
                </table>
              </div>
            </>
          ) : (
            <div className="no-font-selected">
              <Type size={48} />
              <h3>Select a font to preview</h3>
              <p>Choose a font from the list to see its preview</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}