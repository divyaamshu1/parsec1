import { useState, useEffect } from 'react'
import { useLearning } from '../../hooks/useLearning'
import { BookOpen, ChevronRight, CheckCircle, Play, Code, Award } from 'lucide-react'

export default function Tutorials() {
  const { 
    tutorials, 
    activeTutorial,
    activeStep,
    tutorialProgress,
    beginTutorial,
    completeCurrentStep,
    goToNextStep,
    goToPrevStep
  } = useLearning()

  const [code, setCode] = useState('')
  const [showSolution, setShowSolution] = useState(false)

  useEffect(() => {
    if (activeTutorial) {
      setCode(activeTutorial.steps[activeStep]?.code || '')
    }
  }, [activeTutorial, activeStep])

  const handleStepComplete = async () => {
    const completed = await completeCurrentStep(code)
    if (completed) {
      await goToNextStep()
    }
  }

  if (!activeTutorial) {
    return (
      <div className="tutorials">
        <div className="tutorials-header">
          <h2>
            <BookOpen size={20} /> Tutorials
          </h2>
        </div>

        <div className="tutorials-grid">
          {tutorials.map(tutorial => (
            <div key={tutorial.id} className="tutorial-card">
              <div className="tutorial-card-header">
                <h3>{tutorial.title}</h3>
                <span className={`difficulty ${tutorial.difficulty}`}>
                  {tutorial.difficulty}
                </span>
              </div>
              <p className="tutorial-description">{tutorial.description}</p>
              <div className="tutorial-meta">
                <span className="tutorial-language">{tutorial.language}</span>
                <span className="tutorial-duration">{tutorial.duration} min</span>
              </div>
              <button onClick={() => beginTutorial(tutorial.id)}>
                Start Tutorial
              </button>
            </div>
          ))}
        </div>
      </div>
    )
  }

  const currentStep = activeTutorial.steps[activeStep]

  return (
    <div className="tutorial-view">
      <div className="tutorial-sidebar">
        <div className="tutorial-info">
          <h3>{activeTutorial.title}</h3>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${tutorialProgress}%` }} />
          </div>
          <span className="progress-text">{tutorialProgress}% Complete</span>
        </div>

        <div className="tutorial-steps">
          {activeTutorial.steps.map((step, index) => (
            <div
              key={step.id}
              className={`step-item ${index === activeStep ? 'active' : ''} ${step.completed ? 'completed' : ''}`}
              onClick={() => {/* Navigate to step */}}
            >
              <div className="step-icon">
                {step.completed ? (
                  <CheckCircle size={16} />
                ) : index === activeStep ? (
                  <Play size={16} />
                ) : (
                  <div className="step-number">{index + 1}</div>
                )}
              </div>
              <div className="step-info">
                <div className="step-title">{step.title}</div>
                <div className="step-type">{step.stepType ?? 'explanation'}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="tutorial-content">
        <div className="step-header">
          <h2>{currentStep.title}</h2>
          <div className="step-navigation">
            <button onClick={goToPrevStep} disabled={activeStep === 0}>
              Previous
            </button>
            <span>Step {activeStep + 1} of {activeTutorial.steps.length}</span>
            <button 
              onClick={activeStep === activeTutorial.steps.length - 1 ? undefined : handleStepComplete}
              disabled={activeStep === activeTutorial.steps.length - 1}
            >
              Next
            </button>
          </div>
        </div>

        <div className="step-description">
          <div dangerouslySetInnerHTML={{ __html: currentStep.content }} />
        </div>

        {currentStep.hint && (
          <div className="step-hint">
            <strong>💡 Hint:</strong> {currentStep.hint}
          </div>
        )}

        <div className="step-code">
          <div className="code-header">
            <h4>
              <Code size={16} /> Code
            </h4>
            <div className="code-actions">
              {currentStep.solution && (
                <button onClick={() => setShowSolution(!showSolution)}>
                  {showSolution ? 'Hide Solution' : 'Show Solution'}
                </button>
              )}
            </div>
          </div>

          <textarea
            value={code}
            onChange={(e) => setCode(e.target.value)}
            className="code-editor"
            rows={10}
          />

          {showSolution && currentStep.solution && (
            <div className="solution">
              <h5>Solution:</h5>
              <pre>{currentStep.solution}</pre>
            </div>
          )}
        </div>

        <div className="step-actions">
          <button className="primary" onClick={handleStepComplete}>
            {(currentStep.stepType ?? '') === 'quiz' ? 'Submit Answer' : 'Complete Step'}
          </button>
          {(currentStep.stepType ?? '') === 'exercise' && (
            <button>
              Run Code
            </button>
          )}
        </div>
      </div>
    </div>
  )
}