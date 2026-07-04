use crate::config::DoctorConfig;
use crate::error::DoctorResult;
use crate::model::SystemModel;
use crate::model::report::DiagnosticReport;
use crate::model::summary::DiagnosisSummary;
use crate::output::{self, OutputFormat};
use crate::plugin::registry::PluginRegistry;
use crate::scanner;
use clap::Args;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Args)]
pub struct DiagnoseArgs {
    /// Project root directory [default: .]
    pub path: Option<String>,

    /// Output format [default: terminal]
    #[arg(short, long, default_value = "terminal")]
    pub output: String,

    /// Plugin names to enable (repeatable)
    #[arg(short, long)]
    pub plugin: Vec<String>,

    /// Skip AI explanation step
    #[arg(long)]
    pub no_ai: bool,

    /// Force offline mode
    #[arg(long)]
    pub offline: bool,

    /// Timeout in seconds
    #[arg(long, default_value = "120")]
    pub timeout: u64,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn run(args: DiagnoseArgs) -> DoctorResult<()> {
    let start = Instant::now();
    let project_path = PathBuf::from(args.path.as_deref().unwrap_or("."));
    let canonical_path = project_path.canonicalize().unwrap_or_else(|_| project_path.clone());

    // Load config
    let config = DoctorConfig::find_and_load(&canonical_path);

    // Setup plugin registry
    let mut registry = PluginRegistry::new(config.plugins.scan_dirs.clone());
    if let Err(e) = registry.scan() {
        if args.verbose {
            eprintln!("Warning: Plugin scan failed: {e}");
        }
    }
    // Enable plugins from CLI args (take precedence) or config
    if !args.plugin.is_empty() {
        registry.enable_all(&args.plugin);
    } else {
        registry.enable_all(&config.plugins.enabled);
    }

    if args.verbose {
        eprintln!("Scanning project: {}", canonical_path.display());
    }

    // Phase 1: Scan
    let system_overview = scanner::scan_project(&canonical_path)?;
    let project_name = canonical_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if args.verbose {
        eprintln!("  Build tool: {}", system_overview.build_tool);
        eprintln!("  Spring Boot: {:?}", system_overview.spring_boot_version);
        eprintln!("  Starters: {:?}", system_overview.starters);
    }

    // Phase 2: Build model from project sources
    let system_model = crate::model::system_model::build_system_model(&canonical_path)?;

    // Phase 3: Collect evidence
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        crate::error::DoctorError::ConfigError(format!("Failed to create async runtime: {e}"))
    })?;

    let actuator_url = if args.offline { None } else { std::env::var("ACTUATOR_BASE_URL").ok() };

    let evidence = rt.block_on(crate::evidence::collect_all(
        &canonical_path,
        actuator_url.as_deref(),
        args.offline,
    ))?;

    if args.verbose {
        eprintln!("  Evidence collected: {} items", evidence.len());
    }

    // Phase 4: Run diagnostic rules
    let diagnose_output = crate::rule_engine::run_all_rules(&system_model, &evidence)?;

    // Build report
    let summary = DiagnosisSummary::new(
        diagnose_output.rules_executed,
        evidence.len(),
        diagnose_output.severity_counts,
        !args.offline && actuator_url.is_some(),
    );

    let report = DiagnosticReport::new(
        project_name,
        start.elapsed().as_millis() as u64,
        system_overview,
        diagnose_output.issues,
        summary,
    );

    // Phase 5: Output
    let format: OutputFormat =
        args.output.parse().map_err(|e: String| crate::error::DoctorError::ConfigError(e))?;
    output::write_report(&report, format, &mut std::io::stdout())?;

    Ok(())
}
