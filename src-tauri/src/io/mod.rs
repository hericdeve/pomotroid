use std::fs;
use std::io::Write;
use serde::{Deserialize, Serialize};
use crate::db::queries::SessionRow;

/// Magic string embedded in every export file so we can reject foreign JSON.
pub const FORMAT_VERSION: &str = "1.0";

/// Top-level envelope written to / read from `.pomotroid.json` files.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportEnvelope {
    /// Always `"1.0"`. Used to identify and validate the file.
    pub pomotroid_export: String,
    /// ISO-8601 UTC timestamp of when the file was produced.
    pub exported_at: String,
    /// DB schema version the data was exported from (for future migration hints).
    pub schema_version: u32,
    /// The session rows being exported / imported.
    pub sessions: Vec<SessionRow>,
}

/// Write `envelope` as pretty-printed JSON to `path`.
/// Creates or truncates the target file.
pub fn write_export(path: &str, envelope: &ExportEnvelope) -> Result<(), String> {
    let json = serde_json::to_string_pretty(envelope)
        .map_err(|e| format!("Serialization error: {e}"))?;
    let mut file = fs::File::create(path)
        .map_err(|e| format!("Cannot create file '{path}': {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Read and validate a Pomotroid JSON export file from `path`.
/// Returns `Err` if the file cannot be parsed or the magic string is missing/wrong.
pub fn read_import(path: &str) -> Result<ExportEnvelope, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read file '{path}': {e}"))?;
    let envelope: ExportEnvelope = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid JSON: {e}"))?;
    if envelope.pomotroid_export != FORMAT_VERSION {
        return Err(format!(
            "Unrecognised export format '{}'. Expected '{FORMAT_VERSION}'.",
            envelope.pomotroid_export
        ));
    }
    Ok(envelope)
}
