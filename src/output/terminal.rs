use crate::error::DoctorResult;
use crate::model::Severity;
use crate::model::report::DiagnosticReport;
use colored::*;
use std::io::Write;

pub fn write(report: &DiagnosticReport, writer: &mut dyn Write) -> DoctorResult<()> {
    // Header
    writeln!(writer, "{}", "╔══════════════════════════════════════════════╗".bright_blue())?;
    writeln!(
        writer,
        "{}",
        format!("║  Doctor Diagnosis Report                     ║").bright_blue()
    )?;
    writeln!(writer, "{}", format!("║  Project: {:35} ║", report.project_name).bright_blue())?;
    writeln!(
        writer,
        "{}",
        format!("║  Health Score: {}/100                          ║", report.health_score)
            .bright_blue()
    )?;
    writeln!(
        writer,
        "{}",
        format!(
            "║  Duration: {:.1}s                               ║",
            report.duration_ms as f64 / 1000.0
        )
        .bright_blue()
    )?;
    writeln!(writer, "{}", "╚══════════════════════════════════════════════╝".bright_blue())?;
    writeln!(writer)?;

    // System overview
    let boot_ver = report.system_overview.spring_boot_version.as_deref().unwrap_or("unknown");
    let java_ver = report.system_overview.java_version.as_deref().unwrap_or("unknown");
    writeln!(
        writer,
        "System: Spring Boot {} | {} | Java {}",
        boot_ver, report.system_overview.build_tool, java_ver
    )?;
    writeln!(writer)?;

    // Health score bar
    let score = report.health_score;
    let bar_color = if score >= 80 {
        "green"
    } else if score >= 50 {
        "yellow"
    } else {
        "red"
    };
    let bar_len = (score as usize) / 2;
    let bar = "█".repeat(bar_len) + &"░".repeat(50 - bar_len);
    writeln!(writer, "Health: [{}] {}/100", bar.color(bar_color), score)?;
    writeln!(writer)?;

    // Issue summary
    let error_count = report.issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warn_count = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
    let info_count = report.issues.iter().filter(|i| i.severity == Severity::Info).count();

    writeln!(writer, "{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black())?;
    if report.issues.is_empty() {
        writeln!(writer, "{}", "✓ No issues found!".green().bold())?;
    } else {
        writeln!(
            writer,
            "Issues: {} ERROR | {} WARNING | {} INFO",
            error_count.to_string().red().bold(),
            warn_count.to_string().yellow().bold(),
            info_count.to_string().blue().bold(),
        )?;
    }
    writeln!(writer, "{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black())?;
    writeln!(writer)?;

    // Issue list
    for issue in &report.issues {
        let icon = match issue.severity {
            Severity::Error => "🔴".to_string(),
            Severity::Warning => "🟡".to_string(),
            Severity::Info => "🔵".to_string(),
        };
        let severity_str = format!("[{}] [{}]", issue.severity, issue.category);
        let colored_severity = match issue.severity {
            Severity::Error => severity_str.red().bold(),
            Severity::Warning => severity_str.yellow().bold(),
            Severity::Info => severity_str.blue(),
        };

        writeln!(writer, "{} {} {}", icon, colored_severity, issue.title)?;

        // Description (indented)
        if !issue.description.is_empty() {
            writeln!(writer, "   {}", issue.description)?;
        }

        // Evidence (first 3)
        for ev in issue.evidence.iter().take(3) {
            writeln!(
                writer,
                "     {} {}: {}",
                "📎".bright_black(),
                ev.source.dimmed(),
                ev.summary
            )?;
        }
        if issue.evidence.len() > 3 {
            writeln!(writer, "     ... and {} more evidence items", issue.evidence.len() - 3)?;
        }

        // Fix suggestion
        if !issue.fix_suggestion.is_empty() {
            writeln!(writer, "   {} Fix: {}", "💡".green(), issue.fix_suggestion)?;
        }

        writeln!(writer)?;
    }

    // Footer
    writeln!(writer, "{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black())?;
    writeln!(
        writer,
        "Duration: {:.2}s | Rules executed: {} | Evidence: {}",
        report.duration_ms as f64 / 1000.0,
        report.summary.total_rules_executed,
        report.summary.total_evidence_collected,
    )?;

    Ok(())
}
