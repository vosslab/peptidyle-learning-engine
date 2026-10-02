//! Reviewable inventory of author JavaScript libraries served by PLE.

/// Library names approved for isolated author JavaScript.
pub(super) const RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES: &[&str] = &["rdkit"];

pub(super) fn is_recorded_external_javascript_dependency(name: &str) -> bool {
    RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES.contains(&name)
}
