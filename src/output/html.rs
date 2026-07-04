use crate::error::DoctorResult;
use crate::model::report::DiagnosticReport;
use crate::model::Severity;
use std::io::Write;

/// Write diagnostic report as standalone HTML with inline CSS.
pub fn write(report: &DiagnosticReport, writer: &mut dyn Write) -> DoctorResult<()> {
    let score_color = if report.health_score >= 80 {
        "#22c55e"
    } else if report.health_score >= 50 {
        "#eab308"
    } else {
        "#ef4444"
    };

    // Escape project name for HTML
    let project_name = html_escape(&report.project_name);

    writeln!(writer, "<!DOCTYPE html>")?;
    writeln!(writer, "<html lang=\"zh-CN\">")?;
    writeln!(writer, "<head>")?;
    writeln!(writer, "<meta charset=\"UTF-8\">")?;
    writeln!(writer, "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
    writeln!(writer, "<title>Doctor Diagnosis: {project_name}</title>")?;
    writeln!(writer, "<style>")?;
    writeln!(writer, "body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; max-width: 960px; margin: 0 auto; padding: 2rem; background: #f8fafc; color: #1e293b; line-height: 1.6; }}")?;
    writeln!(writer, ".header {{ text-align: center; margin-bottom: 2.5rem; padding: 2rem; background: white; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.08); }}")?;
    writeln!(writer, ".header h1 {{ margin: 0 0 0.5rem 0; font-size: 1.8rem; }}")?;
    writeln!(writer, ".score {{ font-size: 5rem; font-weight: 800; color: {score_color}; margin: 1rem 0; }}")?;
    writeln!(writer, ".meta {{ color: #64748b; font-size: 0.9rem; }}")?;
    writeln!(writer, ".section {{ background: white; border-radius: 12px; padding: 1.5rem 2rem; margin-bottom: 1rem; box-shadow: 0 1px 3px rgba(0,0,0,0.08); }}")?;
    writeln!(writer, ".section h2 {{ margin-top: 0; font-size: 1.3rem; border-bottom: 2px solid #e2e8f0; padding-bottom: 0.5rem; }}")?;
    writeln!(writer, ".issue {{ margin-bottom: 1.5rem; padding-bottom: 1.5rem; border-bottom: 1px solid #f1f5f9; }}")?;
    writeln!(writer, ".issue:last-child {{ border-bottom: none; margin-bottom: 0; }}")?;
    writeln!(writer, ".issue-header {{ display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem; }}")?;
    writeln!(writer, ".badge {{ display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; }}")?;
    writeln!(writer, ".badge-error {{ background: #fef2f2; color: #ef4444; border: 1px solid #fecaca; }}")?;
    writeln!(writer, ".badge-warning {{ background: #fefce8; color: #eab308; border: 1px solid #fef08a; }}")?;
    writeln!(writer, ".badge-info {{ background: #eff6ff; color: #3b82f6; border: 1px solid #bfdbfe; }}")?;
    writeln!(writer, ".evidence {{ background: #f8fafc; border: 1px solid #e2e8f0; padding: 0.4rem 0.75rem; border-radius: 4px; margin: 0.25rem 0; font-size: 0.85rem; font-family: 'SF Mono', 'Fira Code', monospace; }}")?;
    writeln!(writer, ".fix {{ background: #f0fdf4; border: 1px solid #bbf7d0; padding: 0.75rem 1rem; border-radius: 6px; margin-top: 0.75rem; }}")?;
    writeln!(writer, ".fix strong {{ color: #16a34a; }}")?;
    writeln!(writer, ".summary-table {{ width: 100%; border-collapse: collapse; }}")?;
    writeln!(writer, ".summary-table td, .summary-table th {{ padding: 0.5rem 0.75rem; text-align: left; }}")?;
    writeln!(writer, ".summary-table th {{ color: #64748b; font-weight: 500; width: 160px; }}")?;
    writeln!(writer, ".empty-state {{ text-align: center; padding: 3rem; color: #64748b; }}")?;
    writeln!(writer, ".empty-state .icon {{ font-size: 3rem; }}")?;
    writeln!(writer, "</style>")?;
    writeln!(writer, "</head>")?;
    writeln!(writer, "<body>")?;

    // Header
    writeln!(writer, "<div class=\"header\">")?;
    writeln!(writer, "<h1>Doctor Diagnosis Report</h1>")?;
    writeln!(writer, "<p class=\"meta\">Project: <strong>{project_name}</strong></p>")?;
    writeln!(writer, "<div class=\"score\">{}/100</div>", report.health_score)?;
    writeln!(
        writer,
        "<p class=\"meta\">Duration: {:.1}s &nbsp;|&nbsp; Rules: {} &nbsp;|&nbsp; Evidence: {}</p>",
        report.duration_ms as f64 / 1000.0,
        report.summary.total_rules_executed,
        report.summary.total_evidence_collected
    )?;
    writeln!(writer, "</div>")?;

    // System Overview
    writeln!(writer, "<div class=\"section\">")?;
    writeln!(writer, "<h2>System Overview</h2>")?;
    writeln!(writer, "<table class=\"summary-table\">")?;
    writeln!(writer, "<tr><th>Build Tool</th><td>{}</td></tr>", report.system_overview.build_tool)?;
    writeln!(
        writer,
        "<tr><th>Spring Boot</th><td>{}</td></tr>",
        report.system_overview.spring_boot_version.as_deref().unwrap_or("N/A")
    )?;
    writeln!(
        writer,
        "<tr><th>Java</th><td>{}</td></tr>",
        report.system_overview.java_version.as_deref().unwrap_or("N/A")
    )?;
    if !report.system_overview.starters.is_empty() {
        writeln!(
            writer,
            "<tr><th>Starters</th><td>{}</td></tr>",
            report.system_overview.starters.join(", ")
        )?;
    }
    writeln!(writer, "</table>")?;
    writeln!(writer, "</div>")?;

    // Issues
    writeln!(writer, "<div class=\"section\">")?;
    writeln!(writer, "<h2>Issues</h2>")?;

    if report.issues.is_empty() {
        writeln!(
            writer,
            "<div class=\"empty-state\"><div class=\"icon\">✅</div><p>No issues found!</p></div>"
        )?;
    } else {
        for issue in &report.issues {
            let (badge_class, label) = match issue.severity {
                Severity::Error => ("badge-error", "ERROR"),
                Severity::Warning => ("badge-warning", "WARNING"),
                Severity::Info => ("badge-info", "INFO"),
            };
            writeln!(writer, "<div class=\"issue\">")?;
            writeln!(writer, "<div class=\"issue-header\">")?;
            writeln!(writer, "<span class=\"badge {badge_class}\">{label}</span>")?;
            writeln!(
                writer,
                "<span class=\"badge\" style=\"background:#f1f5f9;color:#64748b;\">{}</span>",
                issue.category
            )?;
            writeln!(writer, "<strong>{}</strong>", html_escape(&issue.title))?;
            writeln!(writer, "</div>")?;
            writeln!(writer, "<p>{}</p>", html_escape(&issue.description))?;

            if !issue.evidence.is_empty() {
                writeln!(writer, "<p><strong>Evidence:</strong></p>")?;
                for ev in issue.evidence.iter().take(5) {
                    writeln!(
                        writer,
                        "<div class=\"evidence\"><code>{}</code> — {} <em>({})</em></div>",
                        html_escape(&ev.source),
                        html_escape(&ev.summary),
                        ev.reliability
                    )?;
                }
            }

            if !issue.fix_suggestion.is_empty() {
                writeln!(
                    writer,
                    "<div class=\"fix\"><strong>💡 Fix:</strong> {}</div>",
                    html_escape(&issue.fix_suggestion)
                )?;
            }

            writeln!(writer, "</div>")?;
        }
    }

    writeln!(writer, "</div>")?;
    writeln!(writer, "</body>")?;
    writeln!(writer, "</html>")?;

    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
