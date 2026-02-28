export interface File {
  path: string
  name: string
  content: string
  language: string
  dirty: boolean
  created: number
  modified: number
}

export interface Terminal {
  id: string
  name: string
  content: string
  cwd: string
  process?: any
}

export interface AIProvider {
  id: string
  name: string
  type: 'openai' | 'anthropic' | 'copilot' | 'local'
  available: boolean
  models: string[]
}

export interface CloudService {
  id: string
  name: string
  provider: 'aws' | 'gcp' | 'azure'
  type: 'compute' | 'storage' | 'database' | 'serverless'
  status: 'running' | 'stopped' | 'error' | 'creating'
  region: string
  created: number
}

export interface DatabaseConnection {
  id: string
  name: string
  type: 'postgres' | 'mysql' | 'mongodb' | 'sqlite' | 'redis'
  host: string
  port: number
  database: string
  connected: boolean
}

export interface GitStatus {
  branch: string
  changes: GitChange[]
  ahead: number
  behind: number
}

export interface GitChange {
  path: string
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'untracked'
}

export interface DebugSession {
  id: string
  name: string
  state: 'running' | 'paused' | 'stopped'
  threads: DebugThread[]
}

export interface DebugThread {
  id: number
  name: string
  frames: DebugFrame[]
}

export interface DebugFrame {
  id: number
  name: string
  file: string
  line: number
  column: number
}

export interface Extension {
  id: string
  name: string
  publisher: string
  version: string
  description: string
  enabled: boolean
  installed: boolean
  icon?: string
}

export interface Theme {
  id: string
  name: string
  type: 'dark' | 'light'
  colors: Record<string, string>
  editorColors: Record<string, string>
}

export interface Keybinding {
  id: string
  key: string
  command: string
  when?: string
}

export interface User {
  id: string
  name: string
  email: string
  avatar?: string
}