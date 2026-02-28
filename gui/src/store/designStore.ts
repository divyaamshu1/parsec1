import { create } from 'zustand'

interface Color {
  r: number
  g: number
  b: number
  a: number
}

interface ColorPalette {
  id: string
  name: string
  colors: Color[]
  createdAt: number
}

interface Icon {
  id: string
  name: string
  set: string
  category: string
  svg: string
  tags?: string[]
}

interface Font {
  id: string
  name: string
  family: string
  weight: number
  style: string
  version?: string
  glyphs?: number
  preview?: string
}

interface SVGDoc {
  id: string
  name: string
  svg: string
  width: number
  height: number
}

interface DesignState {
  // State
  palettes: ColorPalette[]
  icons: Icon[]
  iconCategories: string[]
  fonts: Font[]
  svgDocuments: SVGDoc[]
  activeColor: Color | null
  activeIcon: Icon | null
  activeFont: Font | null
  activeSVG: SVGDoc | null
  loading: boolean
  error: string | null

  // Color Actions
  loadPalettes: () => Promise<void>
  createPalette: (name: string, colors: Color[]) => Promise<string>
  deletePalette: (id: string) => Promise<void>
  generatePalette: (base: Color, scheme: 'monochromatic' | 'complementary' | 'triadic' | 'tetradic' | 'analogous') => Promise<Color[]>
  setActiveColor: (color: Color | null) => void

  // Icon Actions
  loadIcons: () => Promise<void>
  loadIconCategories: () => Promise<void>
  searchIcons: (query: string, category?: string) => Promise<Icon[]>
  getIcon: (id: string) => Promise<Icon>
  setActiveIcon: (icon: Icon | null) => void

  // Font Actions
  loadFonts: () => Promise<void>
  loadFont: (id: string) => Promise<Font>
  previewFont: (id: string, text: string, size: number) => Promise<string>
  setActiveFont: (font: Font | null) => void

  // SVG Actions
  createSVG: (name: string, width: number, height: number) => Promise<string>
  loadSVG: (id: string) => Promise<SVGDoc>
  updateSVG: (id: string, svg: string) => Promise<void>
  deleteSVG: (id: string) => Promise<void>
  exportSVG: (id: string, format: 'svg' | 'png' | 'jpg') => Promise<string>
  optimizeSVG: (id: string) => Promise<string>
  setActiveSVG: (svg: SVGDoc | null) => void

  // Utility Actions
  getColorFromImage: (imageData: string, count: number) => Promise<Color[]>
  hexToRgb: (hex: string) => Color | null
  rgbToHex: (r: number, g: number, b: number) => string
}

export const useDesignStore = create<DesignState>((set, get) => ({
  // Initial State
  palettes: [],
  icons: [],
  iconCategories: [],
  fonts: [],
  svgDocuments: [],
  activeColor: null,
  activeIcon: null,
  activeFont: null,
  activeSVG: null,
  loading: false,
  error: null,

  // Color Actions
  loadPalettes: async () => {
    set({ loading: true })
    try {
      // Mock data - replace with actual API call
      const palettes: ColorPalette[] = [
        {
          id: '1',
          name: 'Default Dark',
          colors: [
            { r: 30, g: 30, b: 30, a: 1 },
            { r: 0, g: 122, b: 204, a: 1 },
            { r: 212, g: 212, b: 212, a: 1 },
          ],
          createdAt: Date.now(),
        },
        {
          id: '2',
          name: 'Default Light',
          colors: [
            { r: 255, g: 255, b: 255, a: 1 },
            { r: 0, g: 122, b: 204, a: 1 },
            { r: 51, g: 51, b: 51, a: 1 },
          ],
          createdAt: Date.now(),
        },
      ]
      set({ palettes, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  createPalette: async (name: string, colors: Color[]) => {
    const id = Date.now().toString()
    const newPalette: ColorPalette = {
      id,
      name,
      colors,
      createdAt: Date.now(),
    }
    set(state => ({ palettes: [...state.palettes, newPalette] }))
    return id
  },

  deletePalette: async (id: string) => {
    set(state => ({ palettes: state.palettes.filter(p => p.id !== id) }))
  },

  generatePalette: async (base: Color, scheme: string) => {
    // Mock implementation
    const colors: Color[] = [base]
    
    switch (scheme) {
      case 'complementary':
        colors.push({ r: 255 - base.r, g: 255 - base.g, b: 255 - base.b, a: base.a })
        break
      case 'triadic':
        colors.push({ r: base.g, g: base.b, b: base.r, a: base.a })
        colors.push({ r: base.b, g: base.r, b: base.g, a: base.a })
        break
      case 'analogous':
        colors.push({ r: base.r + 30, g: base.g + 30, b: base.b, a: base.a })
        colors.push({ r: base.r - 30, g: base.g - 30, b: base.b, a: base.a })
        break
    }
    
    return colors
  },

  setActiveColor: (color: Color | null) => {
    set({ activeColor: color })
  },

  // Icon Actions
  loadIcons: async () => {
    set({ loading: true })
    try {
      const icons: Icon[] = [
        { id: '1', name: 'home', set: 'material', category: 'ui', svg: '<svg>...</svg>', tags: ['home', 'house'] },
        { id: '2', name: 'settings', set: 'material', category: 'ui', svg: '<svg>...</svg>', tags: ['settings', 'gear'] },
        { id: '3', name: 'user', set: 'feather', category: 'people', svg: '<svg>...</svg>', tags: ['user', 'person'] },
      ]
      set({ icons, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  loadIconCategories: async () => {
    set({ iconCategories: ['ui', 'people', 'files', 'actions', 'brands'] })
  },

  searchIcons: async (query: string, category?: string) => {
    const { icons } = get()
    return icons.filter(icon => 
      icon.name.includes(query) || 
      icon.tags?.some(tag => tag.includes(query))
    )
  },

  getIcon: async (id: string) => {
    const { icons } = get()
    const icon = icons.find(i => i.id === id)
    if (!icon) throw new Error('Icon not found')
    return icon
  },

  setActiveIcon: (icon: Icon | null) => {
    set({ activeIcon: icon })
  },

  // Font Actions
  loadFonts: async () => {
    set({ loading: true })
    try {
      const fonts: Font[] = [
        { id: '1', name: 'Cascadia Code', family: 'Cascadia Code', weight: 400, style: 'normal', version: '1.0' },
        { id: '2', name: 'Fira Code', family: 'Fira Code', weight: 400, style: 'normal', version: '1.0' },
        { id: '3', name: 'Inter', family: 'Inter', weight: 400, style: 'normal', version: '1.0' },
      ]
      set({ fonts, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  loadFont: async (id: string) => {
    const { fonts } = get()
    const font = fonts.find(f => f.id === id)
    if (!font) throw new Error('Font not found')
    return font
  },

  previewFont: async (id: string, text: string, size: number) => {
    // Mock preview - in real app, this would generate an image
    return 'data:image/png;base64,...'
  },

  setActiveFont: (font: Font | null) => {
    set({ activeFont: font })
  },

  // SVG Actions
  createSVG: async (name: string, width: number, height: number) => {
    const id = Date.now().toString()
    const newSVG: SVGDoc = {
      id,
      name,
      svg: `<svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg"></svg>`,
      width,
      height,
    }
    set(state => ({ svgDocuments: [...state.svgDocuments, newSVG] }))
    return id
  },

  loadSVG: async (id: string) => {
    const { svgDocuments } = get()
    const svg = svgDocuments.find(s => s.id === id)
    if (!svg) throw new Error('SVG not found')
    return svg
  },

  updateSVG: async (id: string, svg: string) => {
    set(state => ({
      svgDocuments: state.svgDocuments.map(doc =>
        doc.id === id ? { ...doc, svg } : doc
      )
    }))
  },

  deleteSVG: async (id: string) => {
    set(state => ({
      svgDocuments: state.svgDocuments.filter(doc => doc.id !== id)
    }))
  },

  exportSVG: async (id: string, format: 'svg' | 'png' | 'jpg') => {
    const { svgDocuments } = get()
    const svg = svgDocuments.find(s => s.id === id)
    return svg?.svg || ''
  },

  optimizeSVG: async (id: string) => {
    const { svgDocuments } = get()
    const svg = svgDocuments.find(s => s.id === id)
    return svg?.svg || ''
  },

  setActiveSVG: (svg: SVGDoc | null) => {
    set({ activeSVG: svg })
  },

  // Utility Actions
  getColorFromImage: async (imageData: string, count: number) => {
    // Mock implementation
    return [
      { r: 0, g: 122, b: 204, a: 1 },
      { r: 30, g: 30, b: 30, a: 1 },
      { r: 212, g: 212, b: 212, a: 1 },
    ]
  },

  hexToRgb: (hex: string) => {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16),
      a: 1
    } : null
  },

  rgbToHex: (r: number, g: number, b: number) => {
    return '#' + [r, g, b].map(x => {
      const hex = x.toString(16)
      return hex.length === 1 ? '0' + hex : hex
    }).join('')
  },
}))