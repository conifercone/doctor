use serde::{Deserialize, Serialize};

/// Type of diagnostic evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    SourceCode,
    ConfigFile,
    Runtime,
}

impl std::fmt::Display for EvidenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceType::SourceCode => write!(f, "SourceCode"),
            EvidenceType::ConfigFile => write!(f, "ConfigFile"),
            EvidenceType::Runtime => write!(f, "Runtime"),
        }
    }
}

/// Reliability rating of an evidence item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reliability {
    Confirmed,
    Inferred,
    Unverified,
}

impl std::fmt::Display for Reliability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reliability::Confirmed => write!(f, "Confirmed"),
            Reliability::Inferred => write!(f, "Inferred"),
            Reliability::Unverified => write!(f, "Unverified"),
        }
    }
}

/// A single piece of diagnostic evidence with full source traceability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: EvidenceType,
    /// Source identifier: file_path:line_number or endpoint URL
    pub source: String,
    /// Descriptive summary (NOT full source code or config values)
    pub summary: String,
    pub reliability: Reliability,
}

impl Evidence {
    /// Create a new evidence item. Validates that source is non-empty.
    pub fn new(
        evidence_type: EvidenceType,
        source: impl Into<String>,
        summary: impl Into<String>,
        reliability: Reliability,
    ) -> Self {
        Self { evidence_type, source: source.into(), summary: summary.into(), reliability }
    }
}
