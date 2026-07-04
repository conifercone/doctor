pub mod actuator;
pub mod config_file;
pub mod source;
pub mod types;

pub use types::{Evidence, EvidenceType, Reliability};

use crate::error::DoctorResult;
use std::path::Path;

/// Orchestrate all evidence collectors and merge results.
pub async fn collect_all(
    project_path: &Path,
    actuator_url: Option<&str>,
    offline: bool,
) -> DoctorResult<Vec<Evidence>> {
    let mut all_evidence = Vec::new();

    // Source code evidence (always available)
    if let Ok(source_evidence) = source::collect(project_path) {
        all_evidence.extend(source_evidence);
    }

    // Config file evidence (always available)
    if let Ok(config_evidence) = config_file::collect(project_path) {
        all_evidence.extend(config_evidence);
    }

    // Runtime evidence (only if not offline and URL is available)
    if !offline {
        if let Some(url) = actuator_url {
            match actuator::collect(url).await {
                Ok(runtime_evidence) => all_evidence.extend(runtime_evidence),
                Err(_) => {
                    // Graceful degradation: runtime evidence is optional
                }
            }
        }
    }

    Ok(all_evidence)
}
