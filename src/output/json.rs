use crate::error::DoctorResult;
use crate::model::report::DiagnosticReport;
use std::io::Write;

/// Write diagnostic report as pretty-printed JSON.
pub fn write(report: &DiagnosticReport, writer: &mut dyn Write) -> DoctorResult<()> {
    let json = serde_json::to_string_pretty(report)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
