//! Deterministic PDF writer.

mod assets;
mod layout;
mod render;

/// Writes a self-contained PDF with actual PNG image XObjects. Input is
/// validated by `PrintExam::build_with_assets`, so no writer fallback changes
/// what a student sees.
pub fn write(exam: &crate::PrintExam, layout: crate::PrintLayout) -> crate::ExportArtifact {
    render::write(exam, layout)
}

/// True when a PNG can be carried faithfully by both deterministic writers.
pub fn png_is_supported(bytes: &[u8]) -> bool {
    assets::png_is_supported(bytes)
}


