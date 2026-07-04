use serde::{Deserialize, Serialize};

/// Confidence level of a diagnostic conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "High"),
            Confidence::Medium => write!(f, "Medium"),
            Confidence::Low => write!(f, "Low"),
        }
    }
}
