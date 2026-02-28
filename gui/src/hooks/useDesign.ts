import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

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
  tags?: string[]
}

interface Icon {
  id: string
  name: string
  set: string
  category: string
  svg: string
  width: number
  height: number
  tags: string[]
  popularity: number
}

interface Font {
  id: string
  name: string
  family: string
  weight: number
  style: string
  version: string
  license: string
  glyphs: number
  preview?: string
}

interface SVGDoc {
  id: string
  name: string
  svg: string
  width: number
  height: number
  viewBox: string
  created: number
  modified: number
}

export function useDesign() {
  const [palettes, setPalettes] = useState<ColorPalette[]>([])
  const [icons, setIcons] = useState<Icon[]>([])
  const [iconCategories, setIconCategories] = useState<string[]>([])
  const [fonts, setFonts] = useState<Font[]>([])
  const [svgDocuments, setSvgDocuments] = useState<SVGDoc[]>([])
  const [activeColor, setActiveColor] = useState<Color | null>(null)
  const [activeIcon, setActiveIcon] = useState<Icon | null>(null)
  const [activeFont, setActiveFont] = useState<Font | null>(null)
  const [activeSVG, setActiveSVG] = useState<SVGDoc | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // ==================== Color Management ====================

  const loadPalettes = useCallback(async () => {
    setLoading(true)
    try {
      const palettes = await invoke('get_color_palettes') as ColorPalette[]
      setPalettes(palettes)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const createPalette = useCallback(async (name: string, colors: Color[], tags?: string[]) => {
    setLoading(true)
    try {
      const id = await invoke('create_color_palette', { name, colors, tags }) as string
      await loadPalettes()
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [loadPalettes])

  const deletePalette = useCallback(async (id: string) => {
    setLoading(true)
    try {
      await invoke('delete_color_palette', { id })
      await loadPalettes()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [loadPalettes])

  const generatePalette = useCallback(async (baseColor: Color, scheme: 'monochromatic' | 'complementary' | 'triadic' | 'tetradic' | 'analogous') => {
    setLoading(true)
    try {
      const colors = await invoke('generate_color_palette', { base: baseColor, scheme }) as Color[]
      return colors
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setLoading(false)
    }
  }, [])

  const getColorFromImage = useCallback(async (imageData: string, count: number = 5) => {
    setLoading(true)
    try {
      const colors = await invoke('extract_colors_from_image', { imageData, count }) as Color[]
      return colors
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setLoading(false)
    }
  }, [])

  // ==================== Icon Management ====================

  const loadIcons = useCallback(async () => {
    setLoading(true)
    try {
      const icons = await invoke('get_icons') as Icon[]
      setIcons(icons)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const loadIconCategories = useCallback(async () => {
    setLoading(true)
    try {
      const categories = await invoke('get_icon_categories') as string[]
      setIconCategories(categories)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const searchIcons = useCallback(async (query: string, category?: string, limit?: number) => {
    setLoading(true)
    try {
      const results = await invoke('search_icons', { query, category, limit }) as Icon[]
      return results
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return []
    } finally {
      setLoading(false)
    }
  }, [])

  const getIcon = useCallback(async (id: string) => {
    setLoading(true)
    try {
      const icon = await invoke('get_icon', { id }) as Icon
      setActiveIcon(icon)
      return icon
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const renderIcon = useCallback(async (id: string, size: number, color?: string) => {
    try {
      const png = await invoke('render_icon', { id, size, color }) as string
      return `data:image/png;base64,${png}`
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  // ==================== Font Management ====================

  const loadFonts = useCallback(async () => {
    setLoading(true)
    try {
      const fonts = await invoke('get_fonts') as Font[]
      setFonts(fonts)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const loadFont = useCallback(async (id: string) => {
    setLoading(true)
    try {
      const font = await invoke('load_font', { id }) as Font
      setActiveFont(font)
      return font
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const previewFont = useCallback(async (id: string, text: string, size: number, weight?: number, style?: string) => {
    try {
      const preview = await invoke('preview_font', { id, text, size, weight, style }) as string
      return `data:image/png;base64,${preview}`
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  const installFont = useCallback(async (fontData: string, family: string) => {
    setLoading(true)
    try {
      const id = await invoke('install_font', { fontData, family }) as string
      await loadFonts()
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [loadFonts])

  // ==================== SVG Management ====================

  const createSVG = useCallback(async (name: string, width: number, height: number) => {
    setLoading(true)
    try {
      const id = await invoke('create_svg', { name, width, height }) as string
      await loadSVG(id)
      return id
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const loadSVG = useCallback(async (id: string) => {
    setLoading(true)
    try {
      const svg = await invoke('load_svg', { id }) as SVGDoc
      setSvgDocuments(prev => {
        const exists = prev.some(doc => doc.id === id)
        if (exists) {
          return prev.map(doc => doc.id === id ? svg : doc)
        }
        return [...prev, svg]
      })
      setActiveSVG(svg)
      return svg
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const updateSVG = useCallback(async (id: string, svg: string) => {
    setLoading(true)
    try {
      await invoke('update_svg', { id, svg })
      setSvgDocuments(prev => prev.map(doc => 
        doc.id === id ? { ...doc, svg, modified: Date.now() } : doc
      ))
      if (activeSVG?.id === id) {
        setActiveSVG(prev => prev ? { ...prev, svg, modified: Date.now() } : null)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [activeSVG])

  const deleteSVG = useCallback(async (id: string) => {
    setLoading(true)
    try {
      await invoke('delete_svg', { id })
      setSvgDocuments(prev => prev.filter(doc => doc.id !== id))
      if (activeSVG?.id === id) {
        setActiveSVG(null)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [activeSVG])

  const exportSVG = useCallback(async (id: string, format: 'svg' | 'png' | 'jpg', scale?: number) => {
    try {
      const data = await invoke('export_svg', { id, format, scale }) as string
      return data
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }, [])

  const optimizeSVG = useCallback(async (id: string) => {
    setLoading(true)
    try {
      const optimized = await invoke('optimize_svg', { id }) as string
      setSvgDocuments(prev => prev.map(doc => 
        doc.id === id ? { ...doc, svg: optimized, modified: Date.now() } : doc
      ))
      if (activeSVG?.id === id) {
        setActiveSVG(prev => prev ? { ...prev, svg: optimized, modified: Date.now() } : null)
      }
      return optimized
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    } finally {
      setLoading(false)
    }
  }, [activeSVG])

  // ==================== Color Utilities ====================

  const hexToRgb = useCallback((hex: string): Color | null => {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16),
      a: 1
    } : null
  }, [])

  const rgbToHex = useCallback((r: number, g: number, b: number): string => {
    return '#' + [r, g, b].map(x => {
      const hex = x.toString(16)
      return hex.length === 1 ? '0' + hex : hex
    }).join('')
  }, [])

  const hslToRgb = useCallback((h: number, s: number, l: number): Color => {
    h /= 360
    s /= 100
    l /= 100
    let r, g, b

    if (s === 0) {
      r = g = b = l
    } else {
      const hue2rgb = (p: number, q: number, t: number) => {
        if (t < 0) t += 1
        if (t > 1) t -= 1
        if (t < 1/6) return p + (q - p) * 6 * t
        if (t < 1/2) return q
        if (t < 2/3) return p + (q - p) * (2/3 - t) * 6
        return p
      }

      const q = l < 0.5 ? l * (1 + s) : l + s - l * s
      const p = 2 * l - q
      r = hue2rgb(p, q, h + 1/3)
      g = hue2rgb(p, q, h)
      b = hue2rgb(p, q, h - 1/3)
    }

    return {
      r: Math.round(r * 255),
      g: Math.round(g * 255),
      b: Math.round(b * 255),
      a: 1
    }
  }, [])

  const rgbToHsl = useCallback((r: number, g: number, b: number): { h: number; s: number; l: number } => {
    r /= 255
    g /= 255
    b /= 255
    const max = Math.max(r, g, b)
    const min = Math.min(r, g, b)
    let h = 0, s = 0
    const l = (max + min) / 2

    if (max !== min) {
      const d = max - min
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
      
      switch (max) {
        case r: h = (g - b) / d + (g < b ? 6 : 0); break
        case g: h = (b - r) / d + 2; break
        case b: h = (r - g) / d + 4; break
      }
      h /= 6
    }

    return {
      h: Math.round(h * 360),
      s: Math.round(s * 100),
      l: Math.round(l * 100)
    }
  }, [])

  const lightenColor = useCallback((color: Color, amount: number = 0.1): Color => {
    const hsl = rgbToHsl(color.r, color.g, color.b)
    const newL = Math.min(100, hsl.l + (amount * 100))
    return hslToRgb(hsl.h, hsl.s, newL)
  }, [rgbToHsl, hslToRgb])

  const darkenColor = useCallback((color: Color, amount: number = 0.1): Color => {
    const hsl = rgbToHsl(color.r, color.g, color.b)
    const newL = Math.max(0, hsl.l - (amount * 100))
    return hslToRgb(hsl.h, hsl.s, newL)
  }, [rgbToHsl, hslToRgb])

  const mixColors = useCallback((color1: Color, color2: Color, weight: number = 0.5): Color => {
    return {
      r: Math.round(color1.r * (1 - weight) + color2.r * weight),
      g: Math.round(color1.g * (1 - weight) + color2.g * weight),
      b: Math.round(color1.b * (1 - weight) + color2.b * weight),
      a: color1.a * (1 - weight) + color2.a * weight
    }
  }, [])

  const getContrastColor = useCallback((color: Color): Color => {
    const luminance = (0.299 * color.r + 0.587 * color.g + 0.114 * color.b) / 255
    return luminance > 0.5 
      ? { r: 0, g: 0, b: 0, a: 1 }
      : { r: 255, g: 255, b: 255, a: 1 }
  }, [])

  // Load initial data
  useEffect(() => {
    loadPalettes()
    loadIcons()
    loadIconCategories()
    loadFonts()
  }, [loadPalettes, loadIcons, loadIconCategories, loadFonts])

  return {
    // State
    palettes,
    icons,
    iconCategories,
    fonts,
    svgDocuments,
    activeColor,
    activeIcon,
    activeFont,
    activeSVG,
    loading,
    error,

    // Color Management
    loadPalettes,
    createPalette,
    deletePalette,
    generatePalette,
    getColorFromImage,
    setActiveColor,
    hexToRgb,
    rgbToHex,
    hslToRgb,
    rgbToHsl,
    lightenColor,
    darkenColor,
    mixColors,
    getContrastColor,

    // Icon Management
    loadIcons,
    loadIconCategories,
    searchIcons,
    getIcon,
    renderIcon,
    setActiveIcon,

    // Font Management
    loadFonts,
    loadFont,
    previewFont,
    installFont,
    setActiveFont,

    // SVG Management
    createSVG,
    loadSVG,
    updateSVG,
    deleteSVG,
    exportSVG,
    optimizeSVG,
    setActiveSVG,
  }
}