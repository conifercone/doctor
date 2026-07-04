use serde::{Deserialize, Serialize};

/// Diagnostic issue category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Bean,
    Config,
    Transaction,
    AutoConfig,
    Startup,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Bean => write!(f, "Bean"),
            Category::Config => write!(f, "Config"),
            Category::Transaction => write!(f, "Transaction"),
            Category::AutoConfig => write!(f, "AutoConfig"),
            Category::Startup => write!(f, "Startup"),
        }
    }
}
