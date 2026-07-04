use crate::ai::summary::build_summary;
use crate::config::DoctorConfig;
use crate::error::{DoctorError, DoctorResult};
use crate::model::report::DiagnosticReport;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct ExplainArgs {
    /// Diagnostic report JSON file
    pub report: Option<String>,

    /// LLM API endpoint URL [env: DOCTOR_LLM_URL]
    #[arg(long)]
    pub api_url: Option<String>,

    /// LLM API key [env: DOCTOR_LLM_KEY]
    #[arg(long)]
    pub api_key: Option<String>,

    /// LLM model name [default: claude-sonnet-4-6]
    #[arg(long, default_value = "claude-sonnet-4-6")]
    pub model: String,

    /// Output language [default: zh-CN]
    #[arg(long, default_value = "zh-CN")]
    pub locale: String,
}

pub fn run(args: ExplainArgs) -> DoctorResult<()> {
    let report_path = match &args.report {
        Some(path) => PathBuf::from(path),
        None => {
            return Err(DoctorError::ConfigError(
                "No report file specified. Usage: doctor explain <report.json>".to_string(),
            ));
        }
    };

    if !report_path.exists() {
        return Err(DoctorError::IoError {
            path: report_path.display().to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Report file not found",
            ),
        });
    }

    let report_json = std::fs::read_to_string(&report_path).map_err(|e| DoctorError::IoError {
        path: report_path.display().to_string(),
        source: e,
    })?;

    let report: DiagnosticReport =
        serde_json::from_str(&report_json).map_err(|e| DoctorError::ParseError {
            file: report_path.display().to_string(),
            message: format!("Invalid diagnostic report JSON: {e}"),
        })?;

    // Build structured summary (no source code, no config values per FR-030)
    let summary = build_summary(&report);

    // Load config
    let config = DoctorConfig::find_and_load(&std::env::current_dir().unwrap_or_default());

    // Resolve API credentials: CLI args > env vars > config file
    let api_url = args
        .api_url
        .or_else(|| std::env::var("DOCTOR_LLM_URL").ok())
        .unwrap_or(config.ai.api_url);

    let api_key = args
        .api_key
        .or_else(|| std::env::var(&config.ai.api_key_env).ok())
        .ok_or_else(|| {
            DoctorError::ConfigError(format!(
                "LLM API key not found. Set {} environment variable or use --api-key",
                config.ai.api_key_env
            ))
        })?;

    let model = args.model;

    // Call LLM
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        DoctorError::ConfigError(format!("Failed to create async runtime: {e}"))
    })?;

    eprintln!(
        "Analyzing {} issues with model: {model}...",
        summary.issues.len()
    );

    let explanation = rt.block_on(crate::ai::explain::explain(
        &summary, &api_url, &api_key, &model,
    ))?;

    println!("{}", explanation.raw_response);
    eprintln!(
        "\n--- Model: {} | Issues analyzed: {}",
        explanation.model_used,
        summary.issues.len()
    );

    Ok(())
}
