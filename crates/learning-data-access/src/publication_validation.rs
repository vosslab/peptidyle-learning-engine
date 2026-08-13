//! Backend-neutral authoring and publication validation.

use crate::{
    AssetDeliveryScope, CreateQtiImportCommand, DraftRecord, ObjectRecord, PublishDraftCommand,
    PublishedProblemRecord, PublishedSourceArtifact, QtiImportItemStatus, QtiImportRef,
    QtiImportRegistry, QtiPublicationPromotion, StoreError, TenantContext, validate_asset_delivery,
};
use objects::{Bucket, ObjectCategory, ObjectKey};
use question_model::{
    DraftQuestionDefinition, ProblemVersionRef, QuestionBackend, QuestionDefinition,
};
use question_model::{DraftQuestionSource, QuestionSource};

mod flat_question;

pub(crate) use flat_question::{
    validate_flat_question_publication, validate_flat_question_publication_grading,
};

/// Profile item identities and per-item result source identities are bounded
/// in Unicode scalar values to match the QTI adapter's safe source-identifier
/// contract. The optional package-level registry identifier intentionally has
/// its separate, smaller metadata bound below.
const MAX_QTI_PROFILE_IDENTIFIER_CHARS: usize = 1_024;
const MAX_FLAT_IMPORT_ARCHIVE_LICENSE_CHARS: usize = 512;
const MAX_FLAT_IMPORT_ARCHIVE_PROVENANCE_CHARS: usize = 2_048;
const FIXED_QTI_PROFILE_DEFAULTS_V1: [(&str, &str, &str); 8] = [
    ("policy", "item", "PLE default applied: unlimited attempts."),
    (
        "policy",
        "item",
        "PLE default applied: immediate full feedback.",
    ),
    ("policy", "item", "PLE default applied: untimed."),
    ("policy", "item", "PLE default applied: en-US."),
    ("policy", "item", "PLE default applied: allRightsReserved."),
    ("policy", "item", "PLE default applied: empty tags."),
    ("policy", "item", "PLE default applied: empty taxonomy."),
    ("policy", "item", "PLE default applied: no feedback."),
];

/// Matches the archive annotation checks on current and published provenance
/// rows. PostgreSQL's `btrim(text)` default trims ASCII spaces, while
/// `char_length(text)` counts Unicode scalar values.
pub(crate) fn flat_import_archive_annotations_are_valid(record: &ObjectRecord) -> bool {
    text_has_btrimmed_char_length(&record.license, MAX_FLAT_IMPORT_ARCHIVE_LICENSE_CHARS)
        && text_has_btrimmed_char_length(
            &record.provenance,
            MAX_FLAT_IMPORT_ARCHIVE_PROVENANCE_CHARS,
        )
}

fn text_has_btrimmed_char_length(value: &str, maximum: usize) -> bool {
    let length = value.trim_matches(' ').chars().count();
    (1..=maximum).contains(&length)
}

pub(crate) fn validate_draft(draft: &DraftRecord) -> Result<(), StoreError> {
    validate_question_policies(&draft.question)?;
    draft
        .question
        .metadata
        .validate_title()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if draft.revises.is_some() && draft.derived_from.is_some() {
        return Err(StoreError::InvalidRecord(
            "draft cannot be both a revision and a new fork".to_string(),
        ));
    }
    Ok(())
}

/// Validates every database-side relationship in a QTI staging registry
/// before its transaction begins. Bytes are object-store authoritative and are
/// intentionally not accepted here.
pub(crate) fn validate_qti_import(command: &CreateQtiImportCommand) -> Result<(), StoreError> {
    const MAX_ITEMS: usize = 1_000;
    const MAX_ITEM_RESULTS: usize = 1_000;
    const MAX_ASSETS: usize = 10_000;
    const MAX_UNSUPPORTED: usize = 1_000;
    const MAX_REGISTRY_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
    let registry = &command.registry;
    if registry.source_format != "qti" {
        return Err(StoreError::InvalidRecord(
            "QTI import has an invalid source format".to_string(),
        ));
    }
    if let Some(identifier) = &registry.source_identifier {
        validate_qti_text("source identifier", identifier, 512)?;
    }
    validate_qti_text("importer", &registry.importer, 160)?;
    validate_qti_text("parse schema", &registry.parse_schema, 160)?;
    validate_qti_text("adapter version", &registry.adapter_version, 160)?;
    if let Some(profile_summary) = &registry.profile_summary {
        validate_qti_import_profile_summary(profile_summary)?;
        if registry.importer != "adapter_qti"
            || registry.parse_schema != profile_summary.profile_id()
        {
            return Err(StoreError::InvalidRecord(
                "QTI profile summary does not match its registry adapter contract".to_string(),
            ));
        }
    }
    if registry.items.len() > MAX_ITEMS {
        return Err(StoreError::InvalidRecord(
            "QTI import has an invalid item count".to_string(),
        ));
    }
    if registry.item_results.is_empty() || registry.item_results.len() > MAX_ITEM_RESULTS {
        return Err(StoreError::InvalidRecord(
            "QTI import has an invalid per-item result count".to_string(),
        ));
    }
    if registry.assets.len() > MAX_ASSETS || registry.unsupported_features.len() > MAX_UNSUPPORTED {
        return Err(StoreError::InvalidRecord(
            "QTI import exceeds bounded registry limits".to_string(),
        ));
    }
    let registry_payload = serde_json::to_vec(registry).map_err(|_| {
        StoreError::InvalidRecord("QTI import registry cannot be serialized".to_string())
    })?;
    if registry_payload.len() > MAX_REGISTRY_PAYLOAD_BYTES {
        return Err(StoreError::InvalidRecord(
            "QTI import registry exceeds the 16 MiB metadata limit".to_string(),
        ));
    }
    validate_workspace_source(&registry.source, registry.reference)?;
    let mut assets = std::collections::BTreeSet::new();
    for asset in &registry.assets {
        validate_workspace_asset(asset, registry.reference)?;
        let ObjectKey::WorkspaceAsset {
            asset: logical_asset,
            ..
        } = &asset.key
        else {
            return Err(StoreError::InvalidRecord(
                "QTI asset is missing its logical identity".to_string(),
            ));
        };
        if !assets.insert(*logical_asset) {
            return Err(StoreError::InvalidRecord(
                "QTI import repeats a logical asset".to_string(),
            ));
        }
    }
    let mut item_ids = std::collections::BTreeSet::new();
    for item in &registry.items {
        validate_qti_profile_identifier("item id", &item.item_id)?;
        if !item_ids.insert(item.item_id.as_str()) {
            return Err(StoreError::InvalidRecord(
                "QTI import repeats an item id".to_string(),
            ));
        }
        let mut item_assets = std::collections::BTreeSet::new();
        for asset in &item.assets {
            if !assets.contains(asset) || !item_assets.insert(*asset) {
                return Err(StoreError::InvalidRecord(
                    "QTI item references a missing or repeated staged asset".to_string(),
                ));
            }
        }
    }
    let mut accepted_result_ids = std::collections::BTreeSet::new();
    for result in &registry.item_results {
        validate_qti_item_result_report(result)?;
        match result.status {
            QtiImportItemStatus::Accepted => {
                let item_id = result.item_id.as_deref().ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "accepted QTI item result is missing its item id".to_string(),
                    )
                })?;
                if result.normalized_sha256.is_none()
                    || !item_ids.contains(item_id)
                    || !accepted_result_ids.insert(item_id)
                {
                    return Err(StoreError::InvalidRecord(
                        "accepted QTI item result does not match one staged item".to_string(),
                    ));
                }
            }
            QtiImportItemStatus::Rejected => {
                if let Some(item_id) = &result.item_id {
                    validate_qti_profile_identifier("rejected item id", item_id)?;
                }
            }
        }
    }
    if accepted_result_ids != item_ids {
        return Err(StoreError::InvalidRecord(
            "QTI item results must account for every staged item".to_string(),
        ));
    }
    if command.item_bindings.len() != registry.items.len() {
        return Err(StoreError::InvalidRecord(
            "every QTI item requires exactly one server-only grading binding".to_string(),
        ));
    }
    let bound = command
        .item_bindings
        .iter()
        .map(|binding| binding.item.item_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if bound.len() != command.item_bindings.len()
        || bound != item_ids
        || command.item_bindings.iter().any(|binding| {
            registry
                .items
                .iter()
                .find(|item| item.item_id == binding.item.item_id)
                != Some(&binding.item)
        })
    {
        return Err(StoreError::InvalidRecord(
            "QTI grading bindings must exactly match immutable item records".to_string(),
        ));
    }
    for feature in &registry.unsupported_features {
        validate_qti_warning(feature)?;
    }
    Ok(())
}

fn validate_qti_item_result_report(result: &crate::QtiImportItemResult) -> Result<(), StoreError> {
    validate_qti_profile_identifier("source item identifier", &result.source_identifier)?;
    if let Some(title) = &result.title
        && (title.trim().is_empty()
            || title.chars().count() > 512
            || title.bytes().any(|byte| byte.is_ascii_control()))
    {
        return Err(StoreError::InvalidRecord(
            "QTI item result title is invalid".to_string(),
        ));
    }
    for (name, diagnostics) in [
        ("diagnostic", &result.diagnostics),
        ("default", &result.defaults),
        ("warning", &result.warnings),
    ] {
        validate_qti_diagnostics("QTI item result", name, diagnostics)?;
    }
    if result.status == QtiImportItemStatus::Rejected
        && (result.normalized_sha256.is_some()
            || (result.diagnostics.is_empty() && result.warnings.is_empty()))
    {
        return Err(StoreError::InvalidRecord(
            "rejected QTI item result needs a diagnostic or warning and no normalized checksum"
                .to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_qti_import_profile_summary(
    summary: &crate::QtiImportProfileSummary,
) -> Result<(), StoreError> {
    summary.validate()?;
    validate_qti_diagnostics("QTI profile summary", "default", summary.defaults())?;
    if summary.defaults().len() != FIXED_QTI_PROFILE_DEFAULTS_V1.len()
        || summary
            .defaults()
            .iter()
            .zip(FIXED_QTI_PROFILE_DEFAULTS_V1)
            .any(|(actual, (code, location, detail))| {
                actual.code != code || actual.location != location || actual.detail != detail
            })
    {
        return Err(StoreError::InvalidRecord(
            "QTI profile summary defaults do not match the fixed v1 policy".to_string(),
        ));
    }
    Ok(())
}

fn validate_qti_diagnostics(
    owner: &str,
    name: &str,
    diagnostics: &[crate::QtiUnsupportedFeature],
) -> Result<(), StoreError> {
    const MAX_DIAGNOSTICS_PER_KIND: usize = 32;

    if diagnostics.len() > MAX_DIAGNOSTICS_PER_KIND {
        return Err(StoreError::InvalidRecord(format!(
            "{owner} exceeds the {name} limit"
        )));
    }
    for diagnostic in diagnostics {
        validate_qti_warning(diagnostic)?;
    }
    Ok(())
}

fn validate_qti_warning(feature: &crate::QtiUnsupportedFeature) -> Result<(), StoreError> {
    validate_qti_text("QTI warning code", &feature.code, 160)?;
    validate_qti_text("QTI warning location", &feature.location, 1_024)?;
    validate_qti_text("QTI warning detail", &feature.detail, 2_048)
}

/// Validates the browser-inaccessible evidence supplied by the dedicated QTI
/// publication route. The caller separately loads `registry` from committed
/// private staging while holding its publication transaction open.
pub(crate) fn validate_qti_publication_promotion(
    context: TenantContext,
    command: &PublishDraftCommand,
    promotion: &QtiPublicationPromotion,
    registry: &QtiImportRegistry,
) -> Result<(), StoreError> {
    let (draft_item, draft_import) = match &command.expected_draft.question.source {
        DraftQuestionSource::Qti { item_id, import_id } => (item_id, import_id),
        _ => {
            return Err(StoreError::InvalidRecord(
                "QTI promotion requires a QTI draft".to_string(),
            ));
        }
    };
    let QuestionSource::Qti {
        item_id,
        package_object,
        package_sha256,
    } = &command.published_source
    else {
        return Err(StoreError::InvalidRecord(
            "QTI promotion requires a QTI published source".to_string(),
        ));
    };
    if item_id != draft_item
        || promotion.staging.tenant != context.tenant_id()
        || promotion.staging.workspace != command.expected_draft.question.workspace
        || promotion.staging.import != *draft_import
        || registry.reference != promotion.staging
        || !registry
            .items
            .iter()
            .any(|item| item.item_id == *draft_item)
    {
        return Err(StoreError::Conflict);
    }
    let artifact = command.source_artifact.as_ref().ok_or_else(|| {
        StoreError::InvalidRecord("QTI promotion requires a copied source artifact".to_string())
    })?;
    if *package_object != artifact.object.id
        || package_sha256 != &artifact.object.sha256.to_string()
        || artifact.object.sha256 != registry.source.sha256
        || artifact.object.size_bytes != registry.source.size_bytes
        || artifact.object.media_type != registry.source.media_type
    {
        return Err(StoreError::Conflict);
    }

    let staged_item = registry
        .items
        .iter()
        .find(|item| item.item_id == *draft_item)
        .expect("checked QTI item is present");
    let expected_assets: std::collections::BTreeMap<_, _> = registry
        .assets
        .iter()
        .filter_map(|asset| match &asset.key {
            ObjectKey::WorkspaceAsset { asset: id, .. } if staged_item.assets.contains(id) => {
                Some((*id, asset))
            }
            _ => None,
        })
        .collect();
    if expected_assets.len() != staged_item.assets.len()
        || promotion.assets.len() != expected_assets.len()
    {
        return Err(StoreError::Conflict);
    }
    let mut actual_assets = std::collections::BTreeSet::new();
    for delivery in &promotion.assets {
        validate_asset_delivery(delivery)?;
        let AssetDeliveryScope::Catalog { asset, reference } = delivery.scope else {
            return Err(StoreError::InvalidRecord(
                "QTI promotion assets must be catalog assets".to_string(),
            ));
        };
        if reference != command.publication || !actual_assets.insert(asset) {
            return Err(StoreError::Conflict);
        }
        let staged = expected_assets.get(&asset).ok_or(StoreError::Conflict)?;
        if delivery.object.sha256 != staged.sha256
            || delivery.object.size_bytes != staged.size_bytes
            || delivery.object.media_type != staged.media_type
        {
            return Err(StoreError::Conflict);
        }
    }
    Ok(())
}

fn validate_qti_text(name: &str, value: &str, max: usize) -> Result<(), StoreError> {
    if value.is_empty() || value.len() > max || value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(StoreError::InvalidRecord(format!("QTI {name} is invalid")));
    }
    Ok(())
}

fn validate_qti_profile_identifier(name: &str, value: &str) -> Result<(), StoreError> {
    if value.is_empty()
        || value.chars().count() > MAX_QTI_PROFILE_IDENTIFIER_CHARS
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StoreError::InvalidRecord(format!("QTI {name} is invalid")));
    }
    Ok(())
}

fn validate_workspace_source(
    record: &ObjectRecord,
    reference: QtiImportRef,
) -> Result<(), StoreError> {
    if record.id != record.key.object_id()
        || record.bucket != Bucket::Content
        || record.key.bucket() != Bucket::Content
        || record.category != ObjectCategory::Source
        || record.key.category() != ObjectCategory::Source
        || record.version.is_some()
        || record.key.version_id().is_some()
        || record.media_type != "application/zip"
        || !matches!(record.key, ObjectKey::WorkspaceSource { tenant, workspace, import, .. }
            if tenant == reference.tenant && workspace == reference.workspace && import == reference.import)
    {
        return Err(StoreError::InvalidRecord(
            "QTI source record is not an exact workspace ZIP".to_string(),
        ));
    }
    validate_object_annotations(record)
}

fn validate_workspace_asset(
    record: &ObjectRecord,
    reference: QtiImportRef,
) -> Result<(), StoreError> {
    if record.id != record.key.object_id()
        || record.bucket != Bucket::Content
        || record.key.bucket() != Bucket::Content
        || record.category != ObjectCategory::Asset
        || record.key.category() != ObjectCategory::Asset
        || record.version.is_some()
        || record.key.version_id().is_some()
        || record.media_type.is_empty()
        || !matches!(record.key, ObjectKey::WorkspaceAsset { tenant, workspace, import, .. }
            if tenant == reference.tenant && workspace == reference.workspace && import == reference.import)
    {
        return Err(StoreError::InvalidRecord(
            "QTI asset record is not an exact workspace asset".to_string(),
        ));
    }
    validate_object_annotations(record)
}

fn validate_object_annotations(record: &ObjectRecord) -> Result<(), StoreError> {
    validate_qti_text("object media type", &record.media_type, 255)?;
    validate_qti_text("object license", &record.license, 512)?;
    validate_qti_text("object provenance", &record.provenance, 2_048)
}

/// Ensures the server-prepared immutable source is for the exact draft being
/// published. This prevents a caller from attaching a snapshot from another
/// backend or iMathAS item while the draft is still tenant-owned.
pub(crate) fn validate_publication_source(
    draft: &DraftRecord,
    source: &question_model::QuestionSource,
) -> Result<(), StoreError> {
    source
        .validate()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if question_model::QuestionBackend::from(&draft.question.source)
        != question_model::QuestionBackend::from(source)
    {
        return Err(StoreError::InvalidRecord(
            "published source backend must match the draft source".to_string(),
        ));
    }
    match (&draft.question.source, source) {
        (
            question_model::DraftQuestionSource::Imathas { provider, item_ref },
            question_model::QuestionSource::Imathas {
                provider: published_provider,
                item_ref: published_item,
                ..
            },
        ) if provider == published_provider && item_ref == published_item => Ok(()),
        (question_model::DraftQuestionSource::Imathas { .. }, _) => Err(StoreError::InvalidRecord(
            "iMathAS publication must pin the draft provider and item in its snapshot".to_string(),
        )),
        (
            question_model::DraftQuestionSource::Qti { item_id, .. },
            question_model::QuestionSource::Qti {
                item_id: published_item,
                ..
            },
        ) if item_id == published_item => Ok(()),
        (question_model::DraftQuestionSource::Qti { .. }, _) => Err(StoreError::InvalidRecord(
            "QTI publication must preserve the staged import item identity".to_string(),
        )),
        (
            question_model::DraftQuestionSource::Native { family },
            question_model::QuestionSource::Native {
                family: published_family,
            },
        ) if family == published_family => Ok(()),
        (question_model::DraftQuestionSource::Native { .. }, _) => Err(StoreError::InvalidRecord(
            "native publication must preserve the draft question family".to_string(),
        )),
        _ => Ok(()),
    }
}

/// Validates the server-prepared source object before publication can create
/// any visible immutable identity.
pub(crate) fn validate_source_artifact(
    publication: ProblemVersionRef,
    source: &question_model::QuestionSource,
    artifact: Option<&PublishedSourceArtifact>,
) -> Result<(), StoreError> {
    let backend = QuestionBackend::from(source);
    let requires_artifact = !matches!(backend, QuestionBackend::Native);
    let Some(artifact) = artifact else {
        return if requires_artifact {
            Err(StoreError::InvalidRecord(
                "source-backed publication requires an immutable source artifact".to_string(),
            ))
        } else {
            Ok(())
        };
    };
    if !requires_artifact {
        return Err(StoreError::InvalidRecord(
            "native publication must not attach a source artifact".to_string(),
        ));
    }
    validate_source_artifact_identity(publication, backend, artifact)?;
    if let question_model::QuestionSource::Imathas {
        snapshot,
        snapshot_sha256,
        ..
    } = source
        && (*snapshot != artifact.object.id
            || snapshot_sha256 != &artifact.object.sha256.to_string())
    {
        return Err(StoreError::InvalidRecord(
            "iMathAS snapshot must match the immutable source artifact".to_string(),
        ));
    }
    if let question_model::QuestionSource::Qti {
        package_object,
        package_sha256,
        ..
    } = source
        && (*package_object != artifact.object.id
            || package_sha256 != &artifact.object.sha256.to_string())
    {
        return Err(StoreError::InvalidRecord(
            "QTI package must match the immutable source artifact".to_string(),
        ));
    }
    Ok(())
}

/// Validates source artifacts for publication. The closed flat families are
/// source-backed even though they use the native adapter: their canonical
/// author source and private grading material must promote together. Other
/// native families remain algorithmic and do not gain an artifact requirement.
pub(crate) fn validate_source_artifact_for_publication(
    publication: ProblemVersionRef,
    source: &question_model::QuestionSource,
    artifact: Option<&PublishedSourceArtifact>,
    has_flat_promotion: bool,
) -> Result<(), StoreError> {
    let backend = QuestionBackend::from(source);
    if backend != QuestionBackend::Native {
        if has_flat_promotion {
            return Err(StoreError::InvalidRecord(
                "flat-question promotion requires a supported native flat family".to_string(),
            ));
        }
        return validate_source_artifact(publication, source, artifact);
    }
    let is_flat_family = matches!(
        source,
        QuestionSource::Native { family } if grading::flat_question::is_flat_question_family(family)
    );
    match (is_flat_family, artifact, has_flat_promotion) {
        (false, None, false) => Ok(()),
        (false, Some(_), false) => Err(StoreError::InvalidRecord(
            "native publication must not attach a source artifact".to_string(),
        )),
        (false, _, true) => Err(StoreError::InvalidRecord(
            "flat-question promotion requires a supported native flat family".to_string(),
        )),
        (true, None, false) => Err(StoreError::InvalidRecord(
            "flat-question publication requires a flat-question promotion".to_string(),
        )),
        (true, Some(_), false) => Err(StoreError::InvalidRecord(
            "flat-question publication requires a flat-question promotion".to_string(),
        )),
        (true, None, true) => Err(StoreError::InvalidRecord(
            "flat-question publication requires a copied source artifact".to_string(),
        )),
        (true, Some(artifact), true) => {
            validate_source_artifact_identity(publication, backend, artifact)
        }
    }
}

/// Validates the state-independent half of optional imported lineage.
///
/// `None` is structurally valid for a manually authored flat question. The
/// backend still fails closed when its locked workspace state contains a
/// current origin. A present selector reaches that locked comparison only
/// after its published archive is revalidated against this publication.
pub(crate) fn validate_flat_import_publication_promotion(
    context: TenantContext,
    publication: ProblemVersionRef,
    promotion: Option<&crate::FlatImportPublicationPromotion>,
) -> Result<(), StoreError> {
    let Some(promotion) = promotion else {
        return Ok(());
    };
    let archive = promotion.published_archive();
    let ObjectKey::PublishedImportArchive {
        tenant,
        problem,
        version,
        import,
        object,
    } = &archive.key
    else {
        return Err(StoreError::InvalidRecord(
            "flat-import publication requires a published archive key".to_string(),
        ));
    };
    let expected_object = objects::published_import_archive_object_id(
        *tenant,
        *problem,
        *version,
        *import,
        archive.sha256,
    );
    if *tenant != context.tenant_id()
        || *problem != publication.problem
        || *version != publication.version
        || *object != expected_object
        || archive.id != expected_object
        || archive.bucket != Bucket::Content
        || archive.key.bucket() != Bucket::Content
        || archive.category != ObjectCategory::Source
        || archive.key.category() != ObjectCategory::Source
        || archive.version != Some(publication.version)
        || archive.key.version_id() != Some(publication.version)
        || archive.media_type != crate::QTI_PROFILE_ARCHIVE_MEDIA_TYPE
        || archive.size_bytes == 0
        || archive.size_bytes > crate::flat_import_provenance::MAX_QTI_PROFILE_ARCHIVE_BYTES
        || !flat_import_archive_annotations_are_valid(archive)
    {
        return Err(StoreError::InvalidRecord(
            "flat-import publication archive does not match the target publication".to_string(),
        ));
    }
    Ok(())
}

/// Checks the object-record half of a source binding. Store resolvers repeat
/// this before returning a decoded database payload.
pub(crate) fn validate_source_artifact_identity(
    publication: ProblemVersionRef,
    backend: QuestionBackend,
    artifact: &PublishedSourceArtifact,
) -> Result<(), StoreError> {
    if artifact.reference != publication || artifact.backend != backend {
        return Err(StoreError::InvalidRecord(
            "source artifact must bind the exact published backend and version".to_string(),
        ));
    }
    let ObjectKey::ProblemSource {
        problem,
        version,
        object,
    } = artifact.object.key
    else {
        return Err(StoreError::InvalidRecord(
            "source artifact must use a published problem-source key".to_string(),
        ));
    };
    if problem != publication.problem
        || version != publication.version
        || object != artifact.object.id
        || artifact.object.bucket != Bucket::Content
        || artifact.object.category != ObjectCategory::Source
        || artifact.object.version != Some(publication.version)
        || artifact.object.key.bucket() != Bucket::Content
        || artifact.object.key.category() != ObjectCategory::Source
        || artifact.object.size_bytes == 0
        || artifact.object.media_type.trim().is_empty()
        || artifact.object.license.trim().is_empty()
        || artifact.object.provenance.trim().is_empty()
    {
        return Err(StoreError::InvalidRecord(
            "source artifact metadata does not match its immutable object key".to_string(),
        ));
    }
    Ok(())
}

/// Enforces published identity agreement before immutable insertion.
pub(crate) fn validate_published(record: &PublishedProblemRecord) -> Result<(), StoreError> {
    if record.question.problem != record.problem || record.question.version != record.version {
        return Err(StoreError::InvalidRecord(
            "published record IDs must match its question definition".to_string(),
        ));
    }
    if record.authors.is_empty() {
        return Err(StoreError::InvalidRecord(
            "published problem must have at least one author".to_string(),
        ));
    }
    validate_question_policies(&record.question)?;
    record
        .question
        .metadata
        .validate_title()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let mut authors = record.authors.clone();
    authors.sort_unstable();
    authors.dedup();
    if authors.len() != record.authors.len() {
        return Err(StoreError::InvalidRecord(
            "published problem authors must be unique".to_string(),
        ));
    }
    if record
        .previous_version
        .is_some_and(|previous| previous == record.version)
    {
        return Err(StoreError::InvalidRecord(
            "published version cannot revise itself".to_string(),
        ));
    }
    Ok(())
}

trait QuestionPolicyView {
    fn attempt_policy(&self) -> &question_model::run_policy::AttemptPolicy;
}

impl QuestionPolicyView for QuestionDefinition {
    fn attempt_policy(&self) -> &question_model::run_policy::AttemptPolicy {
        &self.attempt_policy
    }
}

impl QuestionPolicyView for DraftQuestionDefinition {
    fn attempt_policy(&self) -> &question_model::run_policy::AttemptPolicy {
        &self.attempt_policy
    }
}

fn validate_question_policies(question: &impl QuestionPolicyView) -> Result<(), StoreError> {
    if question.attempt_policy().max_attempts == Some(0) {
        return Err(StoreError::InvalidRecord(
            "question max attempts must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_entry(code: &str) -> crate::QtiUnsupportedFeature {
        crate::QtiUnsupportedFeature {
            code: code.to_string(),
            location: "item".to_string(),
            detail: "A supported, answer-free report detail.".to_string(),
        }
    }

    #[test]
    fn rejected_item_report_requires_an_actionable_answer_free_diagnostic() {
        let mut result = crate::QtiImportItemResult {
            source_identifier: "source-item-1".to_string(),
            title: Some("Cell respiration".to_string()),
            item_id: None,
            normalized_sha256: None,
            status: QtiImportItemStatus::Rejected,
            diagnostics: Vec::new(),
            defaults: vec![report_entry("policy")],
            warnings: Vec::new(),
        };
        assert!(validate_qti_item_result_report(&result).is_err());

        result.diagnostics.push(report_entry("itemShape"));
        validate_qti_item_result_report(&result).expect("safe refusal report validates");
        let serialized = serde_json::to_value(&result).expect("safe report serializes");
        assert_eq!(serialized["title"], "Cell respiration");
        assert_eq!(serialized["diagnostics"][0]["code"], "itemShape");
        assert!(serialized.get("correctChoice").is_none());
        assert!(serialized.get("feedback").is_none());
        assert!(serialized.get("graderBytes").is_none());
    }

    #[test]
    fn profile_identifiers_accept_1024_unicode_scalars_without_truncation() {
        let identifier = "\u{03bb}".repeat(MAX_QTI_PROFILE_IDENTIFIER_CHARS);
        assert_eq!(identifier.chars().count(), MAX_QTI_PROFILE_IDENTIFIER_CHARS);
        assert!(validate_qti_profile_identifier("item id", &identifier).is_ok());
        assert!(validate_qti_profile_identifier("source item identifier", &identifier).is_ok());
        assert_eq!(
            identifier,
            "\u{03bb}".repeat(MAX_QTI_PROFILE_IDENTIFIER_CHARS)
        );
    }

    #[test]
    fn profile_identifiers_reject_1025_unicode_scalars() {
        let identifier = "\u{03bb}".repeat(MAX_QTI_PROFILE_IDENTIFIER_CHARS + 1);
        assert!(validate_qti_profile_identifier("item id", &identifier).is_err());
        assert!(validate_qti_profile_identifier("source item identifier", &identifier).is_err());
    }

    #[test]
    fn package_identifier_keeps_its_independent_512_byte_bound() {
        assert!(validate_qti_text("source identifier", &"p".repeat(512), 512).is_ok());
        assert!(validate_qti_text("source identifier", &"p".repeat(513), 512).is_err());
    }
}
