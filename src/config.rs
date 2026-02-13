use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub vosk: VoskConfig,
    pub assistant: AssistantConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
    #[serde(default)]
    pub time_range: TimeRangeConfig,
    #[serde(default)]
    pub tool: Vec<ToolConfig>,
    #[serde(default)]
    pub tts: TtsConfig,
}

#[derive(Deserialize)]
pub struct TtsConfig {
    #[serde(default = "TtsConfig::default_model_path")]
    pub model_path: String,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            model_path: Self::default_model_path(),
        }
    }
}

impl TtsConfig {
    fn default_model_path() -> String {
        "./ru_RU-irina-medium.onnx.json".into()
    }
}

#[derive(Deserialize)]
pub struct VoskConfig {
    #[serde(default = "VoskConfig::default_model_path")]
    pub model_path: String,
}

impl Default for VoskConfig {
    fn default() -> Self {
        Self {
            model_path: Self::default_model_path(),
        }
    }
}

impl VoskConfig {
    fn default_model_path() -> String {
        "./vosk-model-small-ru-0.22".into()
    }
}

#[derive(Deserialize)]
pub struct AssistantConfig {
    pub wake_word: String,
    #[serde(default = "AssistantConfig::default_stop_words")]
    pub stop_words: Vec<String>,
    #[serde(default = "AssistantConfig::default_system_prompt")]
    pub system_prompt: String,
}

impl AssistantConfig {
    fn default_stop_words() -> Vec<String> {
        vec![
            "стоп".into(),
            "спасибо".into(),
            "хватит".into(),
            "отмена".into(),
        ]
    }
    fn default_system_prompt() -> String {
        "Ты голосовой ассистент. Отвечай кратко на русском языке.".into()
    }
}

#[derive(Deserialize)]
pub struct OpenAiConfig {
    #[serde(default = "OpenAiConfig::default_model")]
    pub model: String,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            model: Self::default_model(),
        }
    }
}

impl OpenAiConfig {
    fn default_model() -> String {
        "gpt-4o-mini".into()
    }
}

#[derive(Deserialize)]
pub struct TimeRangeConfig {
    #[serde(default = "TimeRangeConfig::default_start")]
    pub start_hour: u32,
    #[serde(default = "TimeRangeConfig::default_end")]
    pub end_hour: u32,
}

impl Default for TimeRangeConfig {
    fn default() -> Self {
        Self {
            start_hour: Self::default_start(),
            end_hour: Self::default_end(),
        }
    }
}

impl TimeRangeConfig {
    fn default_start() -> u32 {
        1
    }
    fn default_end() -> u32 {
        18
    }
}

#[derive(Deserialize, Clone)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub params: HashMap<String, ParamConfig>,
    #[serde(default)]
    pub required_params: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct ParamConfig {
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub description: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
        toml::from_str(&content).map_err(|e| format!("{}: {}", path, e))
    }
}
