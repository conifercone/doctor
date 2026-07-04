use crate::error::DoctorResult;
use crate::model::auto_config::{self, AutoConfigModel};
use crate::model::bean_graph::{self, BeanGraph};
use crate::model::config::{self, ConfigModel};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Complete structured description of the target system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemModel {
    pub bean_graph: BeanGraph,
    pub auto_config_model: AutoConfigModel,
    pub config_model: ConfigModel,
}

impl SystemModel {
    pub fn new(
        bean_graph: BeanGraph,
        auto_config_model: AutoConfigModel,
        config_model: ConfigModel,
    ) -> Self {
        Self { bean_graph, auto_config_model, config_model }
    }
}

/// Build a complete system model from the project at the given path.
///
/// Orchestrates bean graph, auto-config model, and config model construction.
/// Each sub-builder is called independently — failure in one does not block others.
pub fn build_system_model(project_path: &Path) -> DoctorResult<SystemModel> {
    let bean_graph = bean_graph::build_bean_graph(project_path)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Bean graph construction failed: {e}");
            BeanGraph::default()
        });

    let auto_config_model = auto_config::build_auto_config_model(project_path)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Auto-config model construction failed: {e}");
            AutoConfigModel::default()
        });

    let config_model = config::build_config_model(project_path)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Config model construction failed: {e}");
            ConfigModel::default()
        });

    Ok(SystemModel::new(bean_graph, auto_config_model, config_model))
}
