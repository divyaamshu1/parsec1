import { useEffect } from 'react'
import { useAppStore } from '../store/appStore'

const darkTheme = {
  background: '#1e1e1e',
  foreground: '#d4d4d4',
  primary: '#007acc',
  secondary: '#6c757d',
  accent: '#9cdcfe',
  success: '#6a9955',
  warning: '#cca700',
  error: '#f48771',
  info: '#3794ff',
  selection: '#264f78',
  border: '#3c3c3c',
  sidebar: '#252526',
  tab: '#2d2d2d',
  tabActive: '#1e1e1e',
  statusBar: '#007acc'
}

const lightTheme = {
  background: '#ffffff',
  foreground: '#333333',
  primary: '#007acc',
  secondary: '#6c757d',
  accent: '#005a9e',
  success: '#2e7d32',
  warning: '#f57c00',
  error: '#d32f2f',
  info: '#1976d2',
  selection: '#add6ff',
  border: '#e0e0e0',
  sidebar: '#f3f3f3',
  tab: '#ececec',
  tabActive: '#ffffff',
  statusBar: '#007acc'
}

export function useTheme() {
  const { theme, setTheme } = useAppStore()

  const toggleTheme = () => {
    setTheme(theme === 'dark' ? 'light' : 'dark')
  }

  const getThemeColors = () => {
    return theme === 'dark' ? darkTheme : lightTheme
  }

  const getCSSVariable = (name: string) => {
    return getComputedStyle(document.documentElement)
      .getPropertyValue(`--${name}`)
      .trim()
  }

  const setCSSVariable = (name: string, value: string) => {
    document.documentElement.style.setProperty(`--${name}`, value)
  }

  useEffect(() => {
    const colors = getThemeColors()
    Object.entries(colors).forEach(([key, value]) => {
      setCSSVariable(key, value)
    })
  }, [theme])

  return {
    theme,
    toggleTheme,
    getThemeColors,
    getCSSVariable,
    setCSSVariable
  }
}