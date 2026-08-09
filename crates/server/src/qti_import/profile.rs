//! Strict profile-package preparation shared by the QTI worker and routes.
//!
//! This module has no HTTP, object-store, or persistence effects. It selects
//! an exact vendor profile before generic compatibility, owns the canonical
//! safe report digest, and keeps accepted mapped items private.

use adapter_qti::profiles::{
    BlackboardQtiImportError, CanvasQtiImportError, QtiMappedItem, QtiProfileDiagnostic,
    QtiProfileId, QtiSafeDiagnostic, QtiSafeItemReport, QtiSafeItemStatus, import_blackboard_qti21,
    import_canvas_qti12,
};
use adapter_qti::{QtiImportIntegrityDigests, QtiImportLimits};
use learning_data_access::{
    PersistedFlatImportProfile, QtiImportItemResult, QtiImportItemStatus, QtiUnsupportedFeature,
};
use objects::Sha256Digest;

/// Answer-free failure classes for strict profile selection and projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QtiProfilePreparationError {
    Ambiguous,
    Contradictory,
    InvalidReport,
}

/// One accepted private mapping plus its canonical integrity evidence.
///
/// This type deliberately implements neither `Debug` nor serialization.
pub(crate) struct PreparedQtiProfileItem {
    mapped_item: QtiMappedItem,
    integrity: QtiImportIntegrityDigests,
}

impl PreparedQtiProfileItem {
    #[allow(dead_code)] // Consumed by the parallel WP-QTI-9 conversion route.
    pub(crate) fn source_identifier(&self) -> &str {
        self.mapped_item.safe_report().source_identifier()
    }

    pub(crate) fn integrity(&self) -> QtiImportIntegrityDigests {
        self.integrity
    }

    pub(crate) fn into_mapped_item(self) -> QtiMappedItem {
        self.mapped_item
    }
}

/// Complete recognized-package result with a safe ordered projection.
///
/// Accepted answer bindings remain inside [`PreparedQtiProfileItem`]. This
/// type deliberately implements neither `Debug` nor serialization.
pub(crate) struct PreparedQtiProfilePackage {
    profile: PersistedFlatImportProfile,
    profile_report_sha256: Sha256Digest,
    package_defaults: Vec<QtiUnsupportedFeature>,
    item_results: Vec<QtiImportItemResult>,
    items: Vec<PreparedQtiProfileItem>,
}

impl PreparedQtiProfilePackage {
    pub(crate) fn profile(&self) -> PersistedFlatImportProfile {
        self.profile
    }

    pub(crate) fn profile_report_sha256(&self) -> Sha256Digest {
        self.profile_report_sha256
    }

    pub(crate) fn package_defaults(&self) -> &[QtiUnsupportedFeature] {
        &self.package_defaults
    }

    pub(crate) fn item_results(&self) -> &[QtiImportItemResult] {
        &self.item_results
    }

    pub(crate) fn into_items(self) -> Vec<PreparedQtiProfileItem> {
        self.items
    }

    /// Consumes the package and returns only the accepted item with the exact
    /// safe registry identity requested by a conversion route.
    #[allow(dead_code)] // Consumed by the parallel WP-QTI-9 conversion route.
    pub(crate) fn into_item(self, source_identifier: &str) -> Option<PreparedQtiProfileItem> {
        self.items
            .into_iter()
            .find(|item| item.source_identifier() == source_identifier)
    }
}

/// Attempts exact Canvas and Blackboard parsing before the caller may use the
/// generic compatibility importer. A recognized package with rejected items
/// is still returned as that vendor profile and never falls through.
pub(crate) fn prepare_qti_profile_package(
    bytes: &[u8],
    limits: QtiImportLimits,
) -> Result<Option<PreparedQtiProfilePackage>, QtiProfilePreparationError> {
    let canvas = import_canvas_qti12(bytes, limits);
    let blackboard = import_blackboard_qti21(bytes, limits);
    match (canvas, blackboard) {
        (Ok(_), Ok(_)) => Err(QtiProfilePreparationError::Ambiguous),
        (Ok(package), _) => prepare_package(
            QtiProfileId::CANVAS,
            package.reports().to_vec(),
            package
                .profile_report_digest_input()
                .map_err(|_| QtiProfilePreparationError::InvalidReport)?,
            package.into_mapped_items(),
        )
        .map(Some),
        (_, Ok(package)) => prepare_package(
            QtiProfileId::BLACKBOARD,
            package.reports().to_vec(),
            package
                .profile_report_digest_input()
                .map_err(|_| QtiProfilePreparationError::InvalidReport)?,
            package.into_mapped_items(),
        )
        .map(Some),
        (Err(canvas), Err(blackboard)) => {
            if contradictory_canvas(canvas) || contradictory_blackboard(blackboard) {
                Err(QtiProfilePreparationError::Contradictory)
            } else {
                Ok(None)
            }
        }
    }
}

fn prepare_package(
    profile: QtiProfileId,
    reports: Vec<QtiSafeItemReport>,
    report_input: adapter_qti::QtiProfileReportDigestInput,
    mapped_items: Vec<QtiMappedItem>,
) -> Result<PreparedQtiProfilePackage, QtiProfilePreparationError> {
    let profile = match profile {
        QtiProfileId::CANVAS => PersistedFlatImportProfile::CanvasQti12V1,
        QtiProfileId::BLACKBOARD => PersistedFlatImportProfile::BlackboardQti21V1,
        QtiProfileId::GENERIC => return Err(QtiProfilePreparationError::InvalidReport),
    };
    let profile_report_sha256 = report_input
        .profile_report_sha256()
        .map_err(|_| QtiProfilePreparationError::InvalidReport)?;
    let package_defaults = report_input
        .defaults
        .iter()
        .map(project_digest_diagnostic)
        .collect();
    let mut accepted = mapped_items.into_iter();
    let mut item_results = Vec::with_capacity(reports.len());
    let mut items = Vec::new();

    for report in &reports {
        let mapped = match report.status() {
            QtiSafeItemStatus::Accepted => Some(
                accepted
                    .next()
                    .ok_or(QtiProfilePreparationError::InvalidReport)?,
            ),
            QtiSafeItemStatus::Rejected => None,
        };
        let normalized_sha256 = mapped
            .as_ref()
            .map(QtiMappedItem::normalized_profile_item_sha256);
        item_results.push(project_item_result(report, normalized_sha256));
        if let Some(mapped_item) = mapped {
            let integrity = mapped_item
                .compute_integrity_digests(&report_input)
                .map_err(|_| QtiProfilePreparationError::InvalidReport)?;
            items.push(PreparedQtiProfileItem {
                mapped_item,
                integrity,
            });
        }
    }
    if accepted.next().is_some() {
        return Err(QtiProfilePreparationError::InvalidReport);
    }

    Ok(PreparedQtiProfilePackage {
        profile,
        profile_report_sha256,
        package_defaults,
        item_results,
        items,
    })
}

fn project_item_result(
    report: &QtiSafeItemReport,
    normalized_sha256: Option<Sha256Digest>,
) -> QtiImportItemResult {
    let accepted = report.status() == QtiSafeItemStatus::Accepted;
    QtiImportItemResult {
        source_identifier: report.source_identifier().to_string(),
        title: report.title().map(str::to_string),
        item_id: accepted.then(|| report.source_identifier().to_string()),
        normalized_sha256,
        status: if accepted {
            QtiImportItemStatus::Accepted
        } else {
            QtiImportItemStatus::Rejected
        },
        diagnostics: report
            .diagnostics()
            .iter()
            .map(project_safe_diagnostic)
            .collect(),
        defaults: report
            .defaults()
            .iter()
            .map(project_safe_diagnostic)
            .collect(),
        warnings: report
            .warnings()
            .iter()
            .map(project_safe_diagnostic)
            .collect(),
    }
}

fn project_safe_diagnostic(diagnostic: &QtiSafeDiagnostic) -> QtiUnsupportedFeature {
    QtiUnsupportedFeature {
        code: diagnostic.code().as_str().to_string(),
        location: diagnostic.location().to_string(),
        detail: diagnostic.detail().to_string(),
    }
}

fn project_digest_diagnostic(diagnostic: &QtiProfileDiagnostic) -> QtiUnsupportedFeature {
    QtiUnsupportedFeature {
        code: diagnostic.code.as_str().to_string(),
        location: diagnostic.location.clone(),
        detail: diagnostic.detail.clone(),
    }
}

fn contradictory_canvas(error: CanvasQtiImportError) -> bool {
    matches!(error, CanvasQtiImportError::Detection(_))
}

fn contradictory_blackboard(error: BlackboardQtiImportError) -> bool {
    matches!(error, BlackboardQtiImportError::Detection(_))
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::*;

    const CANVAS_MANIFEST: &str =
        include_str!("../../../adapters/qti/tests/fixtures/profiles/canvas_positive_manifest.xml");
    const CANVAS_META: &str =
        include_str!("../../../adapters/qti/tests/fixtures/profiles/canvas_assessment_meta.xml");
    const CANVAS_ITEM: &str =
        include_str!("../../../adapters/qti/tests/fixtures/profiles/canvas_positive_item.xml");
    const BLACKBOARD_MANIFEST: &str = include_str!(
        "../../../adapters/qti/tests/fixtures/profiles/blackboard_positive_manifest.xml"
    );
    const BLACKBOARD_META: &str = include_str!(
        "../../../adapters/qti/tests/fixtures/profiles/blackboard_assessment_meta.xml"
    );
    const BLACKBOARD_ITEM: &str =
        include_str!("../../../adapters/qti/tests/fixtures/profiles/blackboard_positive_item.xml");

    fn archive(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (path, contents) in entries {
            zip.start_file(*path, options).expect("fixture entry");
            zip.write_all(contents.as_bytes()).expect("fixture body");
        }
        zip.finish().expect("fixture archive").into_inner()
    }

    fn canvas_archive(item: &str) -> Vec<u8> {
        archive(&[
            ("imsmanifest.xml", CANVAS_MANIFEST),
            ("canvas_qti12_questions/assessment_meta.xml", CANVAS_META),
            ("canvas_qti12_questions/canvas-1.xml", item),
        ])
    }

    fn mixed_canvas_item() -> String {
        let start = CANVAS_ITEM.find("      <item ").expect("item start");
        let end = CANVAS_ITEM[start..]
            .find("      </item>")
            .map(|offset| start + offset + "      </item>".len())
            .expect("item end");
        let rejected = CANVAS_ITEM[start..end]
            .replace("canvas-1", "canvas-2")
            .replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
        CANVAS_ITEM.replacen("    </section>", &format!("{rejected}\n    </section>"), 1)
    }

    #[test]
    fn canvas_projection_preserves_safe_source_order_and_private_item_ownership() {
        let package = prepare_qti_profile_package(
            &canvas_archive(&mixed_canvas_item()),
            QtiImportLimits::default(),
        )
        .expect("recognized profile is internally consistent")
        .expect("Canvas profile is selected");

        assert_eq!(package.profile(), PersistedFlatImportProfile::CanvasQti12V1);
        assert!(matches!(
            package.item_results(),
            [accepted, rejected]
                if accepted.source_identifier == "canvas-1"
                    && accepted.item_id.as_deref() == Some("canvas-1")
                    && accepted.status == QtiImportItemStatus::Accepted
                    && accepted.normalized_sha256.is_some()
                    && accepted.diagnostics.is_empty()
                    && rejected.source_identifier == "canvas-2"
                    && rejected.item_id.is_none()
                    && rejected.status == QtiImportItemStatus::Rejected
                    && rejected.normalized_sha256.is_none()
                    && !rejected.diagnostics.is_empty()
        ));
        let safe = serde_json::to_string(&(package.item_results(), package.package_defaults()))
            .expect("safe registry projection serializes");
        assert!(!safe.contains("blue"));
        assert!(!safe.contains("red"));
        assert_eq!(package.into_items()[0].source_identifier(), "canvas-1");
    }

    #[test]
    fn all_rejected_canvas_retains_canonical_report_digest_and_defaults() {
        let rejected =
            CANVAS_ITEM.replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
        let bytes = canvas_archive(&rejected);
        let first = prepare_qti_profile_package(&bytes, QtiImportLimits::default())
            .expect("recognized refusal remains a report")
            .expect("Canvas profile remains selected");
        let second = prepare_qti_profile_package(&bytes, QtiImportLimits::default())
            .expect("exact retry remains a report")
            .expect("Canvas profile remains selected");

        assert_eq!(
            first.profile_report_sha256(),
            second.profile_report_sha256()
        );
        assert!(!first.package_defaults().is_empty());
        assert!(first.item_results().iter().all(|result| {
            result.status == QtiImportItemStatus::Rejected
                && result.item_id.is_none()
                && !result.diagnostics.is_empty()
        }));
        assert!(first.into_items().is_empty());
    }

    #[test]
    fn blackboard_projection_states_the_points_default_warning() {
        let bytes = archive(&[
            ("imsmanifest.xml", BLACKBOARD_MANIFEST),
            ("qti21_items/assessment_meta.xml", BLACKBOARD_META),
            ("qti21_items/bb-1.xml", BLACKBOARD_ITEM),
        ]);
        let package = prepare_qti_profile_package(&bytes, QtiImportLimits::default())
            .expect("recognized profile is internally consistent")
            .expect("Blackboard profile is selected");

        assert_eq!(
            package.profile(),
            PersistedFlatImportProfile::BlackboardQti21V1
        );
        assert!(matches!(
            package.item_results(),
            [accepted]
                if accepted.status == QtiImportItemStatus::Accepted
                    && accepted.warnings.iter().any(|warning| {
                        warning.code == "points"
                            && warning.location == "points"
                            && warning.detail.contains("default 1.0")
                    })
        ));
    }

    #[test]
    fn mixed_vendor_evidence_refuses_instead_of_falling_through_to_generic() {
        let bytes = canvas_archive(BLACKBOARD_ITEM);
        assert!(matches!(
            prepare_qti_profile_package(&bytes, QtiImportLimits::default()),
            Err(QtiProfilePreparationError::Contradictory)
        ));
    }
}
