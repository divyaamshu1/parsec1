import { useState, useEffect } from 'react'
import { useDesign } from '../../hooks/useDesign'
import { Copy, Check, Save, RefreshCw } from 'lucide-react'

export default function ColorPicker() {
  const { 
    palettes,
    activeColor,
    generatePalette,
    createPalette,
    getColorFromImage
  } = useDesign()

  const [color, setColor] = useState('#007acc')
  const [rgb, setRgb] = useState({ r: 0, g: 122, b: 204 })
  const [hsl, setHsl] = useState({ h: 204, s: 100, l: 40 })
  const [hex, setHex] = useState('#007acc')
  const [copied, setCopied] = useState(false)
  const [harmony, setHarmony] = useState<any[]>([])
  const [paletteName, setPaletteName] = useState('')
  const [imageData, setImageData] = useState<string | null>(null)

  useEffect(() => {
    updateFromHex(color)
  }, [])

  const updateFromHex = (hexColor: string) => {
    setHex(hexColor)
    
    // Convert to RGB
    const r = parseInt(hexColor.slice(1, 3), 16)
    const g = parseInt(hexColor.slice(3, 5), 16)
    const b = parseInt(hexColor.slice(5, 7), 16)
    setRgb({ r, g, b })

    // Convert to HSL
    const r1 = r / 255
    const g1 = g / 255
    const b1 = b / 255
    
    const max = Math.max(r1, g1, b1)
    const min = Math.min(r1, g1, b1)
    const l = (max + min) / 2 * 100
    
    let h = 0, s = 0
    if (max !== min) {
      const d = max - min
      s = d / (1 - Math.abs(2 * l / 100 - 1)) * 100
      
      if (max === r1) {
        h = 60 * (((g1 - b1) / d) % 6)
      } else if (max === g1) {
        h = 60 * ((b1 - r1) / d + 2)
      } else {
        h = 60 * ((r1 - g1) / d + 4)
      }
    }
    
    setHsl({ h: Math.round(h), s: Math.round(s), l: Math.round(l) })

    // Generate harmony colors
    generateHarmony(hexColor)
  }

  const generateHarmony = async (baseColor: string) => {
    const colors = await generatePalette(
      { r: rgb.r, g: rgb.g, b: rgb.b, a: 1 },
      'complementary'
    )
    setHarmony(colors)
  }

  const handleHexChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value
    if (/^#[0-9A-F]{6}$/i.test(val)) {
      setColor(val)
      updateFromHex(val)
    }
  }

  const handleRgbChange = (channel: 'r' | 'g' | 'b', value: number) => {
    const newRgb = { ...rgb, [channel]: Math.min(255, Math.max(0, value)) }
    setRgb(newRgb)
    
    const newHex = '#' + 
      newRgb.r.toString(16).padStart(2, '0') +
      newRgb.g.toString(16).padStart(2, '0') +
      newRgb.b.toString(16).padStart(2, '0')
    
    setColor(newHex)
    updateFromHex(newHex)
  }

  const handleHslChange = (channel: 'h' | 's' | 'l', value: number) => {
    const newHsl = { 
      h: channel === 'h' ? Math.min(360, Math.max(0, value)) : hsl.h,
      s: channel === 's' ? Math.min(100, Math.max(0, value)) : hsl.s,
      l: channel === 'l' ? Math.min(100, Math.max(0, value)) : hsl.l
    }
    setHsl(newHsl)
    
    // Convert HSL to RGB (simplified)
    // This would need proper conversion
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(hex)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleImageUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      const reader = new FileReader()
      reader.onload = async (event) => {
        const data = event.target?.result as string
        setImageData(data)
        const colors = await getColorFromImage(data, 5)
        // Update palette with colors
      }
      reader.readAsDataURL(file)
    }
  }

  const handleSavePalette = async () => {
    if (!paletteName) return
    
    const paletteColors = [hex, ...harmony.map((c: any) => c.hex)]
    await createPalette(paletteName, paletteColors.map(c => ({
      r: parseInt(c.slice(1, 3), 16),
      g: parseInt(c.slice(3, 5), 16),
      b: parseInt(c.slice(5, 7), 16),
      a: 1
    })))
    
    setPaletteName('')
  }

  return (
    <div className="color-picker">
      <div className="color-preview" style={{ backgroundColor: hex }}>
        <div className="color-value">{hex}</div>
      </div>

      <div className="color-inputs">
        <div className="input-group">
          <label>Hex</label>
          <input type="text" value={hex} onChange={handleHexChange} />
          <button onClick={handleCopy}>
            {copied ? <Check size={14} /> : <Copy size={14} />}
          </button>
        </div>

        <div className="input-group">
          <label>RGB</label>
          <input
            type="number"
            value={rgb.r}
            onChange={(e) => handleRgbChange('r', parseInt(e.target.value))}
            min="0"
            max="255"
          />
          <input
            type="number"
            value={rgb.g}
            onChange={(e) => handleRgbChange('g', parseInt(e.target.value))}
            min="0"
            max="255"
          />
          <input
            type="number"
            value={rgb.b}
            onChange={(e) => handleRgbChange('b', parseInt(e.target.value))}
            min="0"
            max="255"
          />
        </div>

        <div className="input-group">
          <label>HSL</label>
          <input
            type="number"
            value={hsl.h}
            onChange={(e) => handleHslChange('h', parseInt(e.target.value))}
            min="0"
            max="360"
          />
          <input
            type="number"
            value={hsl.s}
            onChange={(e) => handleHslChange('s', parseInt(e.target.value))}
            min="0"
            max="100"
          />
          <input
            type="number"
            value={hsl.l}
            onChange={(e) => handleHslChange('l', parseInt(e.target.value))}
            min="0"
            max="100"
          />
        </div>
      </div>

      <div className="harmony-section">
        <h4>Harmony Colors</h4>
        <div className="harmony-colors">
          {harmony.map((c, i) => (
            <div
              key={i}
              className="harmony-color"
              style={{ backgroundColor: c.hex }}
              onClick={() => updateFromHex(c.hex)}
            >
              <span className="harmony-value">{c.hex}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="palette-section">
        <h4>Save to Palette</h4>
        <div className="palette-input">
          <input
            type="text"
            value={paletteName}
            onChange={(e) => setPaletteName(e.target.value)}
            placeholder="Palette name"
          />
          <button onClick={handleSavePalette} disabled={!paletteName}>
            <Save size={14} /> Save
          </button>
        </div>
      </div>

      <div className="image-section">
        <h4>Extract Colors from Image</h4>
        <input
          type="file"
          accept="image/*"
          onChange={handleImageUpload}
        />
      </div>

      <div className="palettes-list">
        <h4>My Palettes</h4>
        {palettes.map(palette => (
          <div key={palette.id} className="palette-item">
            <span className="palette-name">{palette.name}</span>
            <div className="palette-colors">
              {palette.colors.map((c, i) => (
                <div
                  key={i}
                  className="palette-color"
                  style={{ backgroundColor: `rgba(${c.r}, ${c.g}, ${c.b}, ${c.a})` }}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}