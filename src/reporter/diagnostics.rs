/// Whether rendered output lists the scan issues that reduced fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDiagnostics {
    Show,
    Hide,
}

impl ScanDiagnostics {
    /// `Show` when `SENSEZ_SCAN_DIAGNOSTICS` or `SENSEZ_TIMING` is set.
    pub fn from_env() -> Self {
        let requested = std::env::var_os("SENSEZ_SCAN_DIAGNOSTICS").is_some()
            || std::env::var_os("SENSEZ_TIMING").is_some();
        if requested {
            ScanDiagnostics::Show
        } else {
            ScanDiagnostics::Hide
        }
    }
}
