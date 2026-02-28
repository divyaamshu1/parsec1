import { useState, useEffect } from 'react'
import { useAccessibility } from '../../hooks/useAccessibility'
import { Droplets, Eye, EyeOff, RefreshCw, Sliders } from 'lucide-react'

export default function ColorBlind() {
  const { 
    colorBlindEnabled,
    colorBlindMode,
    colorBlindTypes,
    simulationStrength,
    toggleColorBlind,
    setColorBlindMode,
    setSimulationStrength,
    simulateColor,
    correctColor
  } = useAccessibility()

  const [testColor, setTestColor] = useState('#007acc')
  const [simulatedColor, setSimulatedColor] = useState('#007acc')
  const [correctedColor, setCorrectedColor] = useState('#007acc')
  const [showSimulation, setShowSimulation] = useState(true)

  useEffect(() => {
    if (colorBlindEnabled && colorBlindMode) {
      const sim = simulateColor(testColor, colorBlindMode, simulationStrength)
      setSimulatedColor(sim)
      
      const corr = correctColor(testColor, colorBlindMode)
      setCorrectedColor(corr)
    }
  }, [testColor, colorBlindMode, simulationStrength, colorBlindEnabled])

  const types = [
    { id: 'protanopia', name: 'Protanopia (Red-blind)', desc: 'Difficulty perceiving red light' },
    { id: 'protanomaly', name: 'Protanomaly (Red-weak)', desc: 'Reduced sensitivity to red' },
    { id: 'deuteranopia', name: 'Deuteranopia (Green-blind)', desc: 'Difficulty perceiving green light' },
    { id: 'deuteranomaly', name: 'Deuteranomaly (Green-weak)', desc: 'Reduced sensitivity to green' },
    { id: 'tritanopia', name: 'Tritanopia (Blue-blind)', desc: 'Difficulty perceiving blue light' },
    { id: 'tritanomaly', name: 'Tritanomaly (Blue-weak)', desc: 'Reduced sensitivity to blue' },
    { id: 'achromatopsia', name: 'Achromatopsia', desc: 'Complete color blindness' },
    { id: 'achromatomaly', name: 'Achromatomaly', desc: 'Partial color blindness' }
  ]

  return (
    <div className="accessibility-panel color-blind">
      <div className="panel-header">
        <h3>
          <Droplets size={18} /> Color Blindness
        </h3>
        <label className="toggle">
          <input
            type="checkbox"
            checked={colorBlindEnabled}
            onChange={toggleColorBlind}
          />
          <span className="toggle-slider"></span>
        </label>
      </div>

      {colorBlindEnabled && (
        <div className="panel-content">
          <div className="type-selector">
            <h4>Simulation Type</h4>
            <select 
              value={colorBlindMode || 'protanopia'}
              onChange={(e) => setColorBlindMode(e.target.value)}
            >
              {types.map(type => (
                <option key={type.id} value={type.id}>
                  {type.name}
                </option>
              ))}
            </select>
            {colorBlindMode && (
              <p className="type-description">
                {types.find(t => t.id === colorBlindMode)?.desc}
              </p>
            )}
          </div>

          <div className="strength-control">
            <h4>
              <Sliders size={14} /> Simulation Strength
            </h4>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={simulationStrength}
              onChange={(e) => setSimulationStrength(Number(e.target.value))}
            />
            <div className="strength-values">
              <span>Mild</span>
              <span>{(simulationStrength * 100).toFixed(0)}%</span>
              <span>Severe</span>
            </div>
          </div>

          <div className="simulation-preview">
            <h4>Color Preview</h4>
            
            <div className="color-picker">
              <input
                type="color"
                value={testColor}
                onChange={(e) => setTestColor(e.target.value)}
              />
              <span>{testColor}</span>
            </div>

            <div className="preview-grid">
              <div className="preview-card">
                <div className="card-header">
                  <Eye size={14} /> Normal Vision
                </div>
                <div 
                  className="color-box"
                  style={{ backgroundColor: testColor }}
                >
                  <span>{testColor}</span>
                </div>
              </div>

              <div className="preview-card">
                <div className="card-header">
                  <EyeOff size={14} /> Simulated
                </div>
                <div 
                  className="color-box simulated"
                  style={{ backgroundColor: simulatedColor }}
                >
                  <span>{simulatedColor}</span>
                </div>
              </div>

              <div className="preview-card">
                <div className="card-header">
                  <RefreshCw size={14} /> Corrected
                </div>
                <div 
                  className="color-box corrected"
                  style={{ backgroundColor: correctedColor }}
                >
                  <span>{correctedColor}</span>
                </div>
              </div>
            </div>
          </div>

          <div className="simulation-toggle">
            <label>
              <input
                type="checkbox"
                checked={showSimulation}
                onChange={(e) => setShowSimulation(e.target.checked)}
              />
              Apply simulation to UI
            </label>
          </div>

          {showSimulation && (
            <div className="ui-preview">
              <h4>UI Preview</h4>
              <div className="ui-sample">
                <div className="sample-sidebar" />
                <div className="sample-main">
                  <div className="sample-toolbar">
                    <div className="sample-tab active">File 1</div>
                    <div className="sample-tab">File 2</div>
                  </div>
                  <div className="sample-editor">
                    <div className="sample-line">function example() {}</div>
                    <div className="sample-line">const x = 42;</div>
                    <div className="sample-line comment">// This is a comment</div>
                  </div>
                </div>
              </div>
            </div>
          )}

          <div className="info-box">
            <h4>About Color Blindness</h4>
            <p>
              Color blindness affects approximately 1 in 12 men (8%) and 1 in 200 women (0.5%).
              These simulations help you understand how your UI appears to users with different
              types of color vision deficiency.
            </p>
          </div>
        </div>
      )}
    </div>
  )
}