import { useState } from 'react'
import { useAPI } from '../../hooks/useAPI'
import Editor from '@monaco-editor/react'
import { 
  Play, Save, Download, Upload, Plus, X,
  ChevronDown, ChevronRight, Copy, Check,
  Settings, History, Bookmark
} from 'lucide-react'

export default function RESTClient() {
  const { 
    requests,
    collections,
    environments,
    sendRequest,
    saveRequest,
    loadRequest,
    addToCollection,
    setEnvironment
  } = useAPI()

  const [method, setMethod] = useState('GET')
  const [url, setUrl] = useState('')
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>([
    { key: 'Content-Type', value: 'application/json' }
  ])
  const [body, setBody] = useState('')
  const [bodyType, setBodyType] = useState<'none' | 'json' | 'form' | 'raw'>('json')
  const [params, setParams] = useState<Array<{ key: string; value: string }>>([])
  const [auth, setAuth] = useState<{ type: string; token?: string; username?: string; password?: string }>({
    type: 'none'
  })
  const [response, setResponse] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [copied, setCopied] = useState(false)
  const [showHeaders, setShowHeaders] = useState(true)
  const [showParams, setShowParams] = useState(false)

  const methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

  const handleSend = async () => {
    setLoading(true)
    try {
      const result = await sendRequest({
        method,
        url,
        headers: headers.reduce((acc, h) => ({ ...acc, [h.key]: h.value }), {}),
        params: params.reduce((acc, p) => ({ ...acc, [p.key]: p.value }), {}),
        body: bodyType !== 'none' ? body : undefined,
        auth
      })
      setResponse(result)
    } catch (error) {
      setResponse({ error: String(error) })
    } finally {
      setLoading(false)
    }
  }

  const addHeader = () => {
    setHeaders([...headers, { key: '', value: '' }])
  }

  const removeHeader = (index: number) => {
    setHeaders(headers.filter((_, i) => i !== index))
  }

  const addParam = () => {
    setParams([...params, { key: '', value: '' }])
  }

  const removeParam = (index: number) => {
    setParams(params.filter((_, i) => i !== index))
  }

  const copyResponse = () => {
    navigator.clipboard.writeText(JSON.stringify(response, null, 2))
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const formatResponse = () => {
    if (response?.data) {
      try {
        return JSON.stringify(response.data, null, 2)
      } catch {
        return response.data
      }
    }
    return ''
  }

  return (
    <div className="api-client rest">
      <div className="request-bar">
        <select value={method} onChange={(e) => setMethod(e.target.value)}>
          {methods.map(m => (
            <option key={m} value={m}>{m}</option>
          ))}
        </select>
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="Enter request URL"
        />
        <button onClick={handleSend} disabled={loading}>
          <Play size={16} /> Send
        </button>
        <button>
          <Save size={16} /> Save
        </button>
      </div>

      <div className="request-tabs">
        <button 
          className={showParams ? 'active' : ''}
          onClick={() => setShowParams(!showParams)}
        >
          Params
        </button>
        <button 
          className={showHeaders ? 'active' : ''}
          onClick={() => setShowHeaders(!showHeaders)}
        >
          Headers
        </button>
        <button>Auth</button>
        <button>Body</button>
      </div>

      {showParams && (
        <div className="params-editor">
          <div className="editor-header">
            <h4>Query Parameters</h4>
            <button onClick={addParam}>
              <Plus size={14} /> Add
            </button>
          </div>
          {params.map((param, i) => (
            <div key={i} className="param-row">
              <input
                type="text"
                value={param.key}
                onChange={(e) => {
                  const newParams = [...params]
                  newParams[i].key = e.target.value
                  setParams(newParams)
                }}
                placeholder="Key"
              />
              <input
                type="text"
                value={param.value}
                onChange={(e) => {
                  const newParams = [...params]
                  newParams[i].value = e.target.value
                  setParams(newParams)
                }}
                placeholder="Value"
              />
              <button onClick={() => removeParam(i)}>
                <X size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      {showHeaders && (
        <div className="headers-editor">
          <div className="editor-header">
            <h4>Headers</h4>
            <button onClick={addHeader}>
              <Plus size={14} /> Add
            </button>
          </div>
          {headers.map((header, i) => (
            <div key={i} className="header-row">
              <input
                type="text"
                value={header.key}
                onChange={(e) => {
                  const newHeaders = [...headers]
                  newHeaders[i].key = e.target.value
                  setHeaders(newHeaders)
                }}
                placeholder="Key"
              />
              <input
                type="text"
                value={header.value}
                onChange={(e) => {
                  const newHeaders = [...headers]
                  newHeaders[i].value = e.target.value
                  setHeaders(newHeaders)
                }}
                placeholder="Value"
              />
              <button onClick={() => removeHeader(i)}>
                <X size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      <div className="body-editor">
        <div className="editor-header">
          <h4>Request Body</h4>
          <select value={bodyType} onChange={(e) => setBodyType(e.target.value as any)}>
            <option value="none">None</option>
            <option value="json">JSON</option>
            <option value="form">Form Data</option>
            <option value="raw">Raw</option>
          </select>
        </div>
        {bodyType !== 'none' && (
          <Editor
            height="200px"
            defaultLanguage={bodyType === 'json' ? 'json' : 'text'}
            value={body}
            onChange={(value) => setBody(value || '')}
            theme="vs-dark"
            options={{
              fontSize: 13,
              minimap: { enabled: false },
              lineNumbers: 'off'
            }}
          />
        )}
      </div>

      {response && (
        <div className="response-viewer">
          <div className="response-header">
            <div className="response-status">
              <span className={`status-code ${response.status < 400 ? 'success' : 'error'}`}>
                {response.status || 'Error'}
              </span>
              <span className="status-text">{response.statusText || ''}</span>
              <span className="response-time">{response.time}ms</span>
            </div>
            <div className="response-actions">
              <button onClick={copyResponse}>
                {copied ? <Check size={14} /> : <Copy size={14} />}
              </button>
              <button>
                <Download size={14} />
              </button>
            </div>
          </div>
          <div className="response-body">
            <Editor
              height="300px"
              defaultLanguage="json"
              value={formatResponse()}
              theme="vs-dark"
              options={{
                readOnly: true,
                fontSize: 13,
                minimap: { enabled: false }
              }}
            />
          </div>
          {response.headers && (
            <div className="response-headers">
              <h5>Response Headers</h5>
              <table>
                <tbody>
                  {Object.entries(response.headers).map(([key, value]) => (
                    <tr key={key}>
                      <td>{key}</td>
                      <td>{value as string}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  )
}