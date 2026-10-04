//! Reviewable inventory of author JavaScript libraries served by PLE.

/// Library names approved for isolated author JavaScript.
pub(super) const RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES: &[&str] = &["rdkit"];

/// CDN domains approved for those dependencies. Empty means PLE serves them.
pub(super) const RECORDED_EXTERNAL_JAVASCRIPT_CDN_DOMAINS: &[&str] = &[];

pub(super) fn is_recorded_external_javascript_dependency(name: &str) -> bool {
    RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES.contains(&name)
}

/// Empty CDN inventory means PLE serves the recorded libraries.
pub(super) fn recorded_javascript_is_served_by_ple() -> bool {
    RECORDED_EXTERNAL_JAVASCRIPT_CDN_DOMAINS.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_external_javascript_dependencies_and_cdn_domains_are_reviewable() {
        assert_eq!(RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES, &["rdkit"]);
        assert!(RECORDED_EXTERNAL_JAVASCRIPT_CDN_DOMAINS.is_empty());
        assert!(recorded_javascript_is_served_by_ple());
        assert!(is_recorded_external_javascript_dependency("rdkit"));
        assert!(!is_recorded_external_javascript_dependency(
            "cdn.example.org"
        ));
    }
}
