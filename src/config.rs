use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Doctor configuration loaded from .doctor.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DoctorConfig {
    #[serde(default)]
    pub plugins: PluginsConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub diagnosis: DiagnosisConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsConfig {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default = "default_scan_dirs")]
    pub scan_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiConfig {
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    #[serde(default = "default_output_format")]
    pub default_format: String,
    #[serde(default = "default_true")]
    pub color: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosisConfig {
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_issues")]
    pub max_issues: usize,
}

fn default_scan_dirs() -> Vec<PathBuf> {
    vec![dirs_fallback()]
}

fn dirs_fallback() -> PathBuf {
    dirs_next().unwrap_or_else(|| PathBuf::from(".doctor/plugins"))
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".doctor").join("plugins"))
}

fn default_api_url() -> String {
    "https://api.anthropic.com/v1/messages".to_string()
}

fn default_api_key_env() -> String {
    "DOCTOR_LLM_KEY".to_string()
}

fn default_model() -> String {
    "claude-sonnet-4-6".to_string()
}

fn default_output_format() -> String {
    "terminal".to_string()
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    120
}

fn default_max_issues() -> usize {
    100
}

impl DoctorConfig {
    /// Load config from a .doctor.toml file.
    pub fn load(path: &std::path::Path) -> crate::error::DoctorResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::DoctorError::IoError { path: path.display().to_string(), source: e }
        })?;
        toml::from_str(&content).map_err(|e| {
            crate::error::DoctorError::ConfigError(format!(
                "Failed to parse config file {}: {e}",
                path.display()
            ))
        })
    }

    /// Search for .doctor.toml in project root, then user home, then default.
    pub fn find_and_load(project_path: &std::path::Path) -> Self {
        // Try project root first
        let project_config = project_path.join(".doctor.toml");
        if project_config.exists() {
            if let Ok(cfg) = Self::load(&project_config) {
                return cfg;
            }
        }
        // Try user home
        if let Ok(home) = std::env::var("HOME") {
            let home_config = std::path::PathBuf::from(home).join(".doctor.toml");
            if home_config.exists() {
                if let Ok(cfg) = Self::load(&home_config) {
                    return cfg;
                }
            }
        }
        Self::default()
    }
}
