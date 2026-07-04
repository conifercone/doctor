use crate::error::{DoctorError, DoctorResult};
use crate::plugin::traits::{EvidenceCollector, ModelBuilder, RuleProvider, Scanner};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Plugin descriptor loaded from plugin directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source_path: PathBuf,
}

/// Registry that discovers and manages plugins.
pub struct PluginRegistry {
    scan_dirs: Vec<PathBuf>,
    discovered: Vec<PluginDescriptor>,
    enabled_names: HashSet<String>,
    scanners: Vec<Box<dyn Scanner>>,
    model_builders: Vec<Box<dyn ModelBuilder>>,
    evidence_collectors: Vec<Box<dyn EvidenceCollector>>,
    rule_providers: Vec<Box<dyn RuleProvider>>,
}

impl PluginRegistry {
    /// Create a new empty registry.
    pub fn new(scan_dirs: Vec<PathBuf>) -> Self {
        Self {
            scan_dirs,
            discovered: Vec::new(),
            enabled_names: HashSet::new(),
            scanners: Vec::new(),
            model_builders: Vec::new(),
            evidence_collectors: Vec::new(),
            rule_providers: Vec::new(),
        }
    }

    /// Scan plugin directories for available plugins.
    ///
    /// Each plugin directory should contain a `plugin.toml` descriptor file.
    pub fn scan(&mut self) -> DoctorResult<()> {
        self.discovered.clear();

        for dir in &self.scan_dirs.clone() {
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(dir)
                .map_err(|e| DoctorError::IoError { path: dir.display().to_string(), source: e })?
            {
                let entry = entry.map_err(|e| DoctorError::IoError {
                    path: dir.display().to_string(),
                    source: e,
                })?;
                let plugin_dir = entry.path();
                if !plugin_dir.is_dir() {
                    continue;
                }
                let toml_path = plugin_dir.join("plugin.toml");
                if toml_path.exists() {
                    let content = std::fs::read_to_string(&toml_path).map_err(|e| {
                        DoctorError::IoError { path: toml_path.display().to_string(), source: e }
                    })?;
                    let desc: PluginDescriptor =
                        toml::from_str(&content).map_err(|e| DoctorError::PluginError {
                            plugin_name: plugin_dir.display().to_string(),
                            message: format!("Failed to parse plugin.toml: {e}"),
                        })?;
                    self.discovered.push(PluginDescriptor { source_path: plugin_dir, ..desc });
                }
            }
        }
        Ok(())
    }

    /// Enable a plugin by name. Must be called after scan().
    pub fn enable(&mut self, name: &str) {
        self.enabled_names.insert(name.to_string());
    }

    /// Enable plugins from a list of names.
    pub fn enable_all(&mut self, names: &[String]) {
        for name in names {
            self.enabled_names.insert(name.clone());
        }
    }

    /// Get descriptors of discovered plugins.
    pub fn discovered(&self) -> &[PluginDescriptor] {
        &self.discovered
    }

    /// Get names of enabled plugins.
    pub fn enabled(&self) -> &HashSet<String> {
        &self.enabled_names
    }

    /// Check if a plugin is enabled.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled_names.contains(name)
    }

    /// Register a scanner.
    pub fn register_scanner(&mut self, scanner: Box<dyn Scanner>) {
        self.scanners.push(scanner);
    }

    /// Register a model builder.
    pub fn register_model_builder(&mut self, builder: Box<dyn ModelBuilder>) {
        self.model_builders.push(builder);
    }

    /// Register an evidence collector.
    pub fn register_evidence_collector(&mut self, collector: Box<dyn EvidenceCollector>) {
        self.evidence_collectors.push(collector);
    }

    /// Register a rule provider.
    pub fn register_rule_provider(&mut self, provider: Box<dyn RuleProvider>) {
        self.rule_providers.push(provider);
    }

    pub fn scanners(&self) -> &[Box<dyn Scanner>] {
        &self.scanners
    }

    pub fn model_builders(&self) -> &[Box<dyn ModelBuilder>] {
        &self.model_builders
    }

    pub fn evidence_collectors(&self) -> &[Box<dyn EvidenceCollector>] {
        &self.evidence_collectors
    }

    pub fn rule_providers(&self) -> &[Box<dyn RuleProvider>] {
        &self.rule_providers
    }
}
