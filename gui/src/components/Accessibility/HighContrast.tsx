import { useState, useEffect } from 'react'
import { useAccessibility } from '../../hooks/useAccessibility'
import { Eye, Palette, Sun, Moon, Contrast, Check } from 'lucide-react'

export default function HighContrast() {
  const { 
    highContrastEnabled,
    contrastThemes,
    currentTheme,
    toggleHighContrast,
    setHighContrastTheme,
    adjustThemeColors,
    testContrastRatio
  } = useAccessibility()

  const [testForeground, setTestForeground] = useState('#ffffff')
  const [testBackground, setTestBackground] = useState('#000000')
  const [contrastRatio, setContrastRatio] = useState(21)
  const [wcagLevel, setWcagLevel] = useState('AAA')

  useEffect(() => {
    if (currentTheme) {
      setTestForeground(currentTheme.colors.foreground)
      setTestBackground(currentTheme.colors.background)
    }
  }, [currentTheme])

  useEffect(() => {
    const ratio = testContrastRatio(testForeground, testBackground)
    setContrastRatio(ratio)
    
    if (ratio >= 7) {
      setWcagLevel('AAA')
    } else if (ratio >= 4.5) {
      setWcagLevel('AA')
    } else if (ratio >= 3) {
      setWcagLevel('AA Large')
    } else {
      setWcagLevel('Fail')
    }
  }, [testForeground, testBackground, testContrastRatio])

  const themes = [
    { id: 'high-contrast-dark', name: 'High Contrast Dark', icon: Moon },
    { id: 'high-contrast-light', name: 'High Contrast Light', icon: Sun },
    { id: 'inverted', name: 'Inverted', icon: Contrast },
    { id: 'custom', name: 'Custom', icon: Palette }
  ]

  return (
    <div className="accessibility-panel high-contrast">
      <div className="panel-header">
        <h3>
          <Eye size={18} /> High Contrast
        </h3>
        <label className="toggle">
          <input
            type="checkbox"
            checked={highContrastEnabled}
            onChange={toggleHighContrast}
          />
          <span className="toggle-slider"></span>
        </label>
      </div>

      {highContrastEnabled && (
        <div className="panel-content">
          <div className="theme-grid">
            {themes.map(theme => {
              const Icon = theme.icon
              const isActive = currentTheme?.id === theme.id
              
              return (
                <button
                  key={theme.id}
                  className={`theme-card ${isActive ? 'active' : ''}`}
                  onClick={() => setHighContrastTheme(theme.id)}
                >
                  <Icon size={24} />
                  <span>{theme.name}</span>
                  {isActive && <Check size={16} className="check" />}
                </button>
              )
            })}
          </div>

          <div className="preview-section">
            <h4>Preview</h4>
            <div 
              className="preview-box"
              style={{
                backgroundColor: currentTheme?.colors.background || '#000000',
                color: currentTheme?.colors.foreground || '#ffffff',
                borderColor: currentTheme?.colors.border || '#ffffff'
              }}
            >
              <h5>Sample Text</h5>
              <p>The quick brown fox jumps over the lazy dog.</p>
              <code>{'function example() { return true; }'}</code>
              <div className="button-example">
                <button>Button</button>
                <button className="primary">Primary</button>
              </div>
            </div>
          </div>

          <div className="contrast-tester">
            <h4>Contrast Tester</h4>
            <div className="color-inputs">
              <div className="color-input">
                <label>Foreground</label>
                <input
                  type="color"
                  value={testForeground}
                  onChange={(e) => setTestForeground(e.target.value)}
                />
              </div>
              <div className="color-input">
                <label>Background</label>
                <input
                  type="color"
                  value={testBackground}
                  onChange={(e) => setTestBackground(e.target.value)}
                />
              </div>
            </div>

            <div className="contrast-result">
              <div className="ratio">{contrastRatio.toFixed(2)}:1</div>
              <div className={`wcag-level ${wcagLevel.toLowerCase().replace(' ', '-')}`}>
                WCAG {wcagLevel}
              </div>
            </div>

            <div 
              className="contrast-sample"
              style={{
                color: testForeground,
                backgroundColor: testBackground
              }}
            >
              Sample Text
            </div>
          </div>

          <div className="wcag-info">
            <h4>WCAG Compliance</h4>
            <ul>
              <li className={contrastRatio >= 4.5 ? 'pass' : 'fail'}>
                AA: 4.5:1 (normal text)
              </li>
              <li className={contrastRatio >= 3 ? 'pass' : 'fail'}>
                AA: 3:1 (large text)
              </li>
              <li className={contrastRatio >= 7 ? 'pass' : 'fail'}>
                AAA: 7:1 (normal text)
              </li>
              <li className={contrastRatio >= 4.5 ? 'pass' : 'fail'}>
                AAA: 4.5:1 (large text)
              </li>
            </ul>
          </div>
        </div>
      )}
    </div>
  )
}