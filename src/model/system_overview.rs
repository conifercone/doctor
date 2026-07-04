use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildTool {
    Maven,
    Gradle,
}

impl std::fmt::Display for BuildTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTool::Maven => write!(f, "Maven"),
            BuildTool::Gradle => write!(f, "Gradle"),
        }
    }
}

/// High-level technology stack snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOverview {
    pub build_tool: BuildTool,
    pub spring_boot_version: Option<String>,
    pub java_version: Option<String>,
    pub starters: Vec<String>,
    pub module_count: usize,
}

impl SystemOverview {
    pub fn new(
        build_tool: BuildTool,
        spring_boot_version: Option<String>,
        java_version: Option<String>,
        starters: Vec<String>,
        module_count: usize,
    ) -> Self {
        Self { build_tool, spring_boot_version, java_version, starters, module_count }
    }
}
