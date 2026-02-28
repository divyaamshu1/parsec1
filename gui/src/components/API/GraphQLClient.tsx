import { useState } from 'react'
import { useAPI } from '../../hooks/useAPI'
import Editor from '@monaco-editor/react'
import { 
  Play, Save, Download, Upload, Plus, X,
  ChevronDown, ChevronRight, Copy, Check,
  Database, Table, FileJson
} from 'lucide-react'

export default function GraphQLClient() {
  const { 
    graphqlRequests,
    collections,
    environments,
    sendGraphQL,
    saveGraphQLRequest,
    loadGraphQLRequest,
    introspectSchema
  } = useAPI()

  const [url, setUrl] = useState('https://api.example.com/graphql')
  const [query, setQuery] = useState(`query {
  users {
    id
    name
    email
  }
}`)
  const [variables, setVariables] = useState('{\n  \n}')
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>([
    { key: 'Content-Type', value: 'application/json' }
  ])
  const [response, setResponse] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [copied, setCopied] = useState(false)
  const [schema, setSchema] = useState<any>(null)
  const [showSchema, setShowSchema] = useState(false)
  const [selectedField, setSelectedField] = useState<string | null>(null)

  const handleSend = async () => {
    setLoading(true)
    try {
      const result = await sendGraphQL({
        url,
        query,
        variables: variables ? JSON.parse(variables) : undefined,
        headers: headers.reduce((acc, h) => ({ ...acc, [h.key]: h.value }), {})
      })
      setResponse(result)
    } catch (error) {
      setResponse({ errors: [{ message: String(error) }] })
    } finally {
      setLoading(false)
    }
  }

  const handleIntrospect = async () => {
    try {
      const schemaData = await introspectSchema(url, headers.reduce((acc, h) => ({ ...acc, [h.key]: h.value }), {}))
      setSchema(schemaData)
      setShowSchema(true)
    } catch (error) {
      alert('Failed to introspect schema: ' + error)
    }
  }

  const addHeader = () => {
    setHeaders([...headers, { key: '', value: '' }])
  }

  const removeHeader = (index: number) => {
    setHeaders(headers.filter((_, i) => i !== index))
  }

  const copyResponse = () => {
    navigator.clipboard.writeText(JSON.stringify(response, null, 2))
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const formatResponse = () => {
    if (response?.data) {
      return JSON.stringify(response.data, null, 2)
    } else if (response?.errors) {
      return JSON.stringify(response.errors, null, 2)
    }
    return ''
  }

  const insertField = (field: string) => {
    setQuery(prev => prev + '\n  ' + field)
  }

  const renderSchemaType = (type: any, depth: number = 0) => {
    if (!type) return null

    return (
      <div key={type.name} className="schema-type" style={{ paddingLeft: depth * 20 }}>
        <div 
          className="schema-type-header"
          onClick={() => setSelectedField(selectedField === type.name ? null : type.name)}
        >
          {type.fields ? (
            selectedField === type.name ? <ChevronDown size={14} /> : <ChevronRight size={14} />
          ) : (
            <FileJson size={14} />
          )}
          <span className="type-name">{type.name}</span>
          <span className="type-kind">{type.kind}</span>
        </div>

        {selectedField === type.name && type.fields && (
          <div className="schema-fields">
            {type.fields.map((field: any) => (
              <div key={field.name} className="schema-field">
                <div className="field-header">
                  <span className="field-name">{field.name}</span>
                  <span className="field-type">{field.type?.name || field.type?.kind}</span>
                  <button 
                    className="insert-btn"
                    onClick={() => insertField(field.name)}
                  >
                    Insert
                  </button>
                </div>
                {field.args && field.args.length > 0 && (
                  <div className="field-args">
                    {field.args.map((arg: any) => (
                      <div key={arg.name} className="arg-item">
                        <span className="arg-name">{arg.name}:</span>
                        <span className="arg-type">{arg.type?.name}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    )
  }

  return (
    <div className="api-client graphql">
      <div className="request-bar">
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="GraphQL endpoint URL"
        />
        <button onClick={handleIntrospect}>
          <Database size={16} /> Schema
        </button>
        <button onClick={handleSend} disabled={loading}>
          <Play size={16} /> Send
        </button>
        <button>
          <Save size={16} /> Save
        </button>
      </div>

      <div className="graphql-container">
        <div className="query-editor-panel">
          <div className="editor-header">
            <h4>Query</h4>
          </div>
          <Editor
            height="200px"
            defaultLanguage="graphql"
            value={query}
            onChange={(value) => setQuery(value || '')}
            theme="vs-dark"
            options={{
              fontSize: 13,
              minimap: { enabled: false },
              lineNumbers: 'on',
              scrollBeyondLastLine: false
            }}
          />

          <div className="editor-header">
            <h4>Variables</h4>
          </div>
          <Editor
            height="150px"
            defaultLanguage="json"
            value={variables}
            onChange={(value) => setVariables(value || '')}
            theme="vs-dark"
            options={{
              fontSize: 13,
              minimap: { enabled: false },
              lineNumbers: 'off',
              scrollBeyondLastLine: false
            }}
          />

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
        </div>

        {showSchema && schema && (
          <div className="schema-panel">
            <div className="schema-header">
              <h4>Schema</h4>
              <button onClick={() => setShowSchema(false)}>✕</button>
            </div>
            <div className="schema-content">
              <div className="schema-section">
                <h5>Query</h5>
                {renderSchemaType(schema.queryType)}
              </div>
              {schema.mutationType && (
                <div className="schema-section">
                  <h5>Mutation</h5>
                  {renderSchemaType(schema.mutationType)}
                </div>
              )}
              {schema.subscriptionType && (
                <div className="schema-section">
                  <h5>Subscription</h5>
                  {renderSchemaType(schema.subscriptionType)}
                </div>
              )}
            </div>
          </div>
        )}

        <div className="response-panel">
          <div className="response-header">
            <h4>Response</h4>
            <div className="response-actions">
              <button onClick={copyResponse}>
                {copied ? <Check size={14} /> : <Copy size={14} />}
              </button>
              <button>
                <Download size={14} />
              </button>
            </div>
          </div>
          <div className="response-content">
            <Editor
              height="calc(100vh - 600px)"
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
        </div>
      </div>
    </div>
  )
}