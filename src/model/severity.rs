use serde::{Deserialize, Serialize};
use std::fmt;

/// Issue severity level with health score deduction values.
/// Variants are ordered by severity: Info < Warning < Error (derived).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational — deducts 1 point
    Info,
    /// Potential risk — deducts 3 points
    Warning,
    /// Critical issue — deducts 10 points from health score
    Error,
}

impl Severity {
    /// Health score points deducted per issue of this severity.
    pub fn deduction(self) -> u8 {
        match self {
            Severity::Error => 10,
            Severity::Warning => 3,
            Severity::Info => 1,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}
