use crate::error::DoctorResult;
use crate::model::report::DiagnosticReport;
use crate::model::Severity;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;

#[derive(Serialize)]
struct SarifLog {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    information_uri: String,
    rules: Vec<SarifRule>,
}

#[derive(Serialize)]
struct SarifRule {
    id: String,
    name: String,
    short_description: SarifMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    help_uri: Option<String>,
}

#[derive(Serialize)]
struct SarifResult {
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize, Default)]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize, Default)]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<SarifRegion>,
}

#[derive(Serialize, Default)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_column: Option<u32>,
}

/// Write diagnostic report as SARIF v2.1.0 compliant JSON.
pub fn write(report: &DiagnosticReport, writer: &mut dyn Write) -> DoctorResult<()> {
    let mut seen_rules = HashSet::new();
    let mut rules = Vec::new();

    // Collect unique rules from issues
    for issue in &report.issues {
        let rule_id = format!("{}-{}", issue.category, issue.id);
        if seen_rules.insert(rule_id.clone()) {
            rules.push(SarifRule {
                id: rule_id,
                name: issue.title.clone(),
                short_description: SarifMessage {
                    text: truncate(&issue.description, 200),
                },
                help_uri: None,
            });
        }
    }

    let results: Vec<SarifResult> = report
        .issues
        .iter()
        .map(|issue| {
            let level = match issue.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "note",
            };

            let locations = issue
                .evidence
                .first()
                .map(|ev| {
                    let (uri, line) = if let Some(colon_pos) = ev.source.rfind(':') {
                        let (path, rest) = ev.source.split_at(colon_pos);
                        let line_num = rest[1..].parse::<u32>().ok();
                        (path.to_string(), line_num)
                    } else {
                        (ev.source.clone(), None)
                    };
                    SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation { uri },
                            region: line.map(|l| SarifRegion {
                                start_line: Some(l),
                                start_column: Some(1),
                            }),
                        },
                    }
                })
                .unwrap_or_default();

            SarifResult {
                rule_id: format!("{}-{}", issue.category, issue.id),
                level: level.to_string(),
                message: SarifMessage {
                    text: format!(
                        "[{}] {}: {}",
                        issue.severity, issue.title, issue.description
                    ),
                },
                locations: vec![locations],
            }
        })
        .collect();

    let sarif = SarifLog {
        schema: "https://json.schemastore.org/sarif-2.1.0.json".to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "Doctor".to_string(),
                    information_uri: "https://github.com/conifercone/doctor".to_string(),
                    rules,
                },
            },
            results,
        }],
    };

    let json = serde_json::to_string_pretty(&sarif)?;
    writeln!(writer, "{json}")?;

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}
