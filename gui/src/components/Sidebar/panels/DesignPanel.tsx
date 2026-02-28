import { useState } from 'react'
import { 
  Palette, Droplets, Type, Image, 
  Layers, Grid, Download, Plus
} from 'lucide-react'

export default function DesignPanel() {
  const [colors] = useState([
    { name: 'Primary', value: '#007acc' },
    { name: 'Secondary', value: '#6c757d' },
    { name: 'Accent', value: '#9cdcfe' },
    { name: 'Success', value: '#6a9955' },
    { name: 'Warning', value: '#cca700' },
    { name: 'Error', value: '#f48771' },
  ])

  const [fonts] = useState([
    { name: 'Cascadia Code', type: 'monospace' },
    { name: 'Fira Code', type: 'monospace' },
    { name: 'Inter', type: 'sans-serif' },
    { name: 'Roboto', type: 'sans-serif' },
  ])

  const [icons] = useState([
    { name: 'home', category: 'ui' },
    { name: 'settings', category: 'ui' },
    { name: 'user', category: 'people' },
    { name: 'folder', category: 'files' },
  ])

  return (
    <div className="design-panel">
      <div className="panel-tabs">
        <button className="active">
          <Palette size={14} /> Colors
        </button>
        <button>
          <Type size={14} /> Fonts
        </button>
        <button>
          <Image size={14} /> Icons
        </button>
        <button>
          <Layers size={14} /> Assets
        </button>
      </div>

      <div className="colors-section">
        <div className="section-header">
          <h4>Color Palette</h4>
          <button>
            <Plus size={14} />
          </button>
        </div>
        <div className="color-grid">
          {colors.map(color => (
            <div key={color.name} className="color-item">
              <div 
                className="color-preview"
                style={{ backgroundColor: color.value }}
              />
              <div className="color-info">
                <span className="color-name">{color.name}</span>
                <span className="color-value">{color.value}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="fonts-section">
        <h4>Fonts</h4>
        {fonts.map(font => (
          <div key={font.name} className="font-item">
            <span className="font-name" style={{ fontFamily: font.name }}>
              {font.name}
            </span>
            <span className="font-type">{font.type}</span>
          </div>
        ))}
      </div>

      <div className="icons-section">
        <h4>Icons</h4>
        <div className="icon-grid">
          {icons.map(icon => (
            <div key={icon.name} className="icon-item">
              <div className="icon-preview">
                <Image size={24} />
              </div>
              <span className="icon-name">{icon.name}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="design-actions">
        <button>
          <Download size={14} /> Export
        </button>
        <button>
          <Grid size={14} /> Variables
        </button>
      </div>
    </div>
  )
}