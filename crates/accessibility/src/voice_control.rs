//! Voice control system for hands-free operation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{RwLock, mpsc};
use tokio::time;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};

#[cfg(feature = "voice-control")]
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream, StreamConfig,
};
#[cfg(feature = "voice-control")]
use hound::WavWriter;

use crate::{Result, AccessibilityError, AccessibilityConfig};

/// Voice command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub id: String,
    pub phrase: String,
    pub command: String,
    pub context: CommandContext,
    pub args: Vec<String>,
    pub confidence: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Command context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandContext {
    Global,
    Editor,
    Terminal,
    FileExplorer,
    Debug,
    Search,
    Settings,
    Custom(String),
}

impl Default for CommandContext {
    fn default() -> Self {
        CommandContext::Global
    }
}

/// Voice profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub language: String,
    pub wake_word: Option<String>,
    pub commands: Vec<VoiceCommandDefinition>,
    pub sensitivity: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Voice command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommandDefinition {
    pub id: String,
    pub phrase: String,
    pub command: String,
    pub context: CommandContext,
    pub args_template: Vec<String>,
    pub enabled: bool,
}

/// Wake word
#[derive(Debug, Clone)]
pub struct WakeWord {
    pub phrase: String,
    pub sensitivity: f32,
    pub requires_pause: bool,
}

/// Speech recognizer
pub struct SpeechRecognizer {
    /// Is listening
    listening: Arc<RwLock<bool>>,
    /// Wake word detected
    wake_word_detected: Arc<RwLock<bool>>,
    /// Commands
    commands: Arc<RwLock<HashMap<String, VoiceCommandDefinition>>>,
    /// Current context
    current_context: Arc<RwLock<CommandContext>>,
    /// Configuration
    config: AccessibilityConfig,
    /// Command channel
    command_tx: mpsc::UnboundedSender<VoiceCommand>,
    command_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<VoiceCommand>>>>,
    #[cfg(feature = "voice-control")]
    stream: Option<Stream>,
}

impl SpeechRecognizer {
    /// Create new speech recognizer
    pub fn new(config: AccessibilityConfig) -> Result<Self> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        #[cfg(feature = "voice-control")]
        let stream = None;

        Ok(Self {
            listening: Arc::new(RwLock::new(false)),
            wake_word_detected: Arc::new(RwLock::new(false)),
            commands: Arc::new(RwLock::new(HashMap::new())),
            current_context: Arc::new(RwLock::new(CommandContext::Global)),
            config,
            command_tx,
            command_rx: Arc::new(RwLock::new(Some(command_rx))),
            #[cfg(feature = "voice-control")]
            stream,
        })
    }

    /// Start listening
    pub async fn start_listening(&self) -> Result<()> {
        *self.listening.write().await = true;
        
        #[cfg(feature = "voice-control")]
        {
            self.start_microphone_stream().await?;
        }

        info!("Voice control listening started");
        Ok(())
    }

    /// Stop listening
    pub async fn stop_listening(&self) -> Result<()> {
        *self.listening.write().await = false;
        *self.wake_word_detected.write().await = false;

        #[cfg(feature = "voice-control")]
        {
            if let Some(stream) = &self.stream {
                stream.pause()?;
            }
        }

        info!("Voice control listening stopped");
        Ok(())
    }

    /// Check if listening
    pub async fn is_listening(&self) -> bool {
        *self.listening.read().await
    }

    /// Start microphone stream
    #[cfg(feature = "voice-control")]
    async fn start_microphone_stream(&self) -> Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| AccessibilityError::MicrophoneError("No input device found".to_string()))?;

        let config = device.default_input_config()
            .map_err(|e| AccessibilityError::MicrophoneError(format!("Failed to get config: {}", e)))?;

        info!("Using input device: {}", device.name()?);
        info!("Default input config: {:?}", config);

        let err_fn = |err| eprintln!("Audio stream error: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    self.process_audio_f32(data);
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    self.process_audio_i16(data);
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    self.process_audio_u16(data);
                },
                err_fn,
                None,
            )?,
        };

        stream.play()?;
        
        let mut self_mut = self;
        self_mut.stream = Some(stream);

        Ok(())
    }

    /// Process audio data (f32)
    #[cfg(feature = "voice-control")]
    fn process_audio_f32(&self, data: &[f32]) {
        // This would integrate with speech recognition library
        // For now, just detect amplitude
        let rms = (data.iter().map(|&x| x * x).sum::<f32>() / data.len() as f32).sqrt();
        
        if rms > 0.01 {
            // Audio detected
        }
    }

    /// Process audio data (i16)
    #[cfg(feature = "voice-control")]
    fn process_audio_i16(&self, data: &[i16]) {
        // Convert to f32 for processing
        let data_f32: Vec<f32> = data.iter().map(|&x| x as f32 / 32768.0).collect();
        self.process_audio_f32(&data_f32);
    }

    /// Process audio data (u16)
    #[cfg(feature = "voice-control")]
    fn process_audio_u16(&self, data: &[u16]) {
        // Convert to f32 for processing
        let data_f32: Vec<f32> = data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0).collect();
        self.process_audio_f32(&data_f32);
    }

    /// Add command
    pub async fn add_command(&self, definition: VoiceCommandDefinition) {
        self.commands.write().await.insert(definition.id.clone(), definition);
    }

    /// Remove command
    pub async fn remove_command(&self, id: &str) {
        self.commands.write().await.remove(id);
    }

    /// Set current context
    pub async fn set_context(&self, context: CommandContext) {
        *self.current_context.write().await = context;
    }

    /// Get current context
    pub async fn current_context(&self) -> CommandContext {
        self.current_context.read().await.clone()
    }

    /// Recognize command from text
    pub async fn recognize_command(&self, text: &str, confidence: f32) -> Option<VoiceCommand> {
        let commands = self.commands.read().await;
        let context = self.current_context.read().await;
        let text_lower = text.to_lowercase();

        for cmd in commands.values() {
            if !cmd.enabled {
                continue;
            }

            // Check context match
            if cmd.context != CommandContext::Global && cmd.context != *context {
                continue;
            }

            // Check phrase match
            if text_lower.contains(&cmd.phrase.to_lowercase()) {
                // Extract arguments
                let mut args = Vec::new();
                for template in &cmd.args_template {
                    if let Some(arg) = self.extract_argument(&text_lower, template) {
                        args.push(arg);
                    }
                }

                return Some(VoiceCommand {
                    id: uuid::Uuid::new_v4().to_string(),
                    phrase: cmd.phrase.clone(),
                    command: cmd.command.clone(),
                    context: cmd.context.clone(),
                    args,
                    confidence,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        None
    }

    /// Extract argument from text
    fn extract_argument(&self, text: &str, template: &str) -> Option<String> {
        // Simple extraction - in production, use NLP
        let patterns: Vec<&str> = template.split("{}").collect();
        if patterns.len() == 2 {
            if let Some(start) = text.find(patterns[0]) {
                let start_idx = start + patterns[0].len();
                if let Some(end) = text[start_idx..].find(patterns[1]) {
                    return Some(text[start_idx..start_idx + end].to_string());
                }
            }
        }
        None
    }

    /// Simulate wake word detection
    async fn check_wake_word(&self, text: &str, wake_word: &str) -> bool {
        text.to_lowercase().contains(&wake_word.to_lowercase())
    }
}

/// Voice control
pub struct VoiceControl {
    /// Recognizer
    recognizer: Arc<SpeechRecognizer>,
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Wake word
    wake_word: Arc<RwLock<Option<WakeWord>>>,
    /// Command history
    history: Arc<RwLock<Vec<VoiceCommand>>>,
    /// Configuration
    config: AccessibilityConfig,
    /// Command processor
    processor_tx: mpsc::UnboundedSender<VoiceCommand>,
    processor_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<VoiceCommand>>>>,
}

impl VoiceControl {
    /// Create new voice control
    pub async fn new(config: AccessibilityConfig) -> Result<Self> {
        let recognizer = Arc::new(SpeechRecognizer::new(config.clone())?);
        let (processor_tx, processor_rx) = mpsc::unbounded_channel();

        let control = Self {
            recognizer,
            enabled: Arc::new(RwLock::new(false)),
            wake_word: Arc::new(RwLock::new(config.default_wake_word.clone().map(|w| WakeWord {
                phrase: w,
                sensitivity: 0.7,
                requires_pause: true,
            }))),
            history: Arc::new(RwLock::new(Vec::with_capacity(100))),
            config,
            processor_tx,
            processor_rx: Arc::new(RwLock::new(Some(processor_rx))),
        };

        // Start command processor
        control.start_processor().await;

        Ok(control)
    }

    /// Start command processor
    async fn start_processor(&self) {
        let history = self.history.clone();
        let mut processor_rx = self.processor_rx.write().await.take().unwrap();

        tokio::spawn(async move {
            while let Some(command) = processor_rx.recv().await {
                // Add to history
                history.write().await.push(command.clone());
                if history.read().await.len() > 100 {
                    history.write().await.remove(0);
                }

                // Process command (would execute actual commands)
                info!("Voice command: {:?}", command);
            }
        });
    }

    /// Enable voice control
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
        if let Err(e) = self.recognizer.start_listening().await {
            warn!("Failed to start voice recognition: {}", e);
        }
    }

    /// Disable voice control
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
        if let Err(e) = self.recognizer.stop_listening().await {
            warn!("Failed to stop voice recognition: {}", e);
        }
    }

    /// Check if enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set wake word
    pub async fn set_wake_word(&self, phrase: &str) {
        *self.wake_word.write().await = Some(WakeWord {
            phrase: phrase.to_string(),
            sensitivity: 0.7,
            requires_pause: true,
        });
    }

    /// Get wake word
    pub async fn wake_word(&self) -> Option<WakeWord> {
        self.wake_word.read().await.clone()
    }

    /// Process recognized text
    pub async fn process_text(&self, text: &str, confidence: f32) -> Result<()> {
        if !self.is_enabled().await {
            return Ok(());
        }

        // Check wake word if set
        if let Some(wake) = self.wake_word.read().await.as_ref() {
            if !self.recognizer.check_wake_word(text, &wake.phrase).await {
                return Ok(());
            }
        }

        // Recognize command
        if let Some(command) = self.recognizer.recognize_command(text, confidence).await {
            self.processor_tx.send(command)?;
        }

        Ok(())
    }

    /// Add custom command
    pub async fn add_command(&self, definition: VoiceCommandDefinition) {
        self.recognizer.add_command(definition).await;
    }

    /// Remove command
    pub async fn remove_command(&self, id: &str) {
        self.recognizer.remove_command(id).await;
    }

    /// Set context
    pub async fn set_context(&self, context: CommandContext) {
        self.recognizer.set_context(context).await;
    }

    /// Get command history
    pub async fn history(&self, limit: Option<usize>) -> Vec<VoiceCommand> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get available commands
    pub async fn commands(&self) -> Vec<VoiceCommandDefinition> {
        // This would need to expose the commands from recognizer
        Vec::new()
    }
}

/// Voice feedback
pub struct VoiceFeedback {
    /// Is enabled
    enabled: Arc<RwLock<bool>>,
    /// Confirmations on/off
    confirmations: Arc<RwLock<bool>>,
    /// Volume (0.0-1.0)
    volume: Arc<RwLock<f32>>,
}

impl VoiceFeedback {
    /// Create new voice feedback
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(true)),
            confirmations: Arc::new(RwLock::new(true)),
            volume: Arc::new(RwLock::new(1.0)),
        }
    }

    /// Enable feedback
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    /// Disable feedback
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    /// Play confirmation tone
    pub async fn play_confirmation(&self) -> Result<()> {
        if !*self.enabled.read().await || !*self.confirmations.read().await {
            return Ok(());
        }

        // Play beep
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::{source::SineWave, Source, OutputStream};
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|_| AccessibilityError::VoiceError("No audio output".to_string()))?;
            
            let source = SineWave::new(440.0)
                .take_duration(Duration::from_millis(100))
                .amplify(*self.volume.read().await as f32);
            
            stream_handle.play_raw(source.convert_samples())?;
        }

        Ok(())
    }

    /// Play error tone
    pub async fn play_error(&self) -> Result<()> {
        if !*self.enabled.read().await {
            return Ok(());
        }

        // Play error beep
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::{source::SineWave, Source, OutputStream};
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|_| AccessibilityError::VoiceError("No audio output".to_string()))?;
            
            let source = SineWave::new(220.0)
                .take_duration(Duration::from_millis(200))
                .amplify(*self.volume.read().await as f32);
            
            stream_handle.play_raw(source.convert_samples())?;
        }

        Ok(())
    }

    /// Set confirmations enabled
    pub async fn set_confirmations(&self, enabled: bool) {
        *self.confirmations.write().await = enabled;
    }

    /// Set volume
    pub async fn set_volume(&self, volume: f32) {
        *self.volume.write().await = volume.clamp(0.0, 1.0);
    }
}

impl Default for VoiceFeedback {
    fn default() -> Self {
        Self::new()
    }
}