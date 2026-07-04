//! Report output formatters.
//!
//! Supports multiple output formats: terminal (colored), JSON,
//! Markdown, HTML, and SARIF. All formatters include sensitive
//! information masking.

pub mod html;
pub mod json;
pub mod markdown;
pub mod mask;
pub mod sarif;
pub mod terminal;

use crate::error::DoctorResult;
use crate::model::report::DiagnosticReport;
use std::io;

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Terminal,
    Json,
    Markdown,
    Html,
    Sarif,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" => Ok(Self::Terminal),
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            "sarif" => Ok(Self::Sarif),
            _ => Err(format!(
                "Unknown output format: {s}. Valid: terminal, json, markdown, html, sarif"
            )),
        }
    }
}

/// Mask sensitive information in a diagnostic report (mutates in place).
fn mask_report(report: &mut DiagnosticReport) {
    for issue in &mut report.issues {
        issue.title = mask::mask_sensitive(&issue.title);
        issue.description = mask::mask_sensitive(&issue.description);
        issue.fix_suggestion = mask::mask_sensitive(&issue.fix_suggestion);
        for ev in &mut issue.evidence {
            ev.summary = mask::mask_sensitive(&ev.summary);
        }
    }
}

/// Write a diagnostic report in the selected format.
///
/// Sensitive information is masked before writing.
pub fn write_report(
    report: &DiagnosticReport,
    format: OutputFormat,
    writer: &mut dyn io::Write,
) -> DoctorResult<()> {
    let mut masked = report.clone();
    mask_report(&mut masked);
    match format {
        OutputFormat::Terminal => terminal::write(&masked, writer),
        OutputFormat::Json => json::write(&masked, writer),
        OutputFormat::Markdown => markdown::write(&masked, writer),
        OutputFormat::Html => html::write(&masked, writer),
        OutputFormat::Sarif => sarif::write(&masked, writer),
    }
}
