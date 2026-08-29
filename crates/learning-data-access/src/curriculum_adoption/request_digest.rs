//! Canonical request bindings for private curriculum-adoption receipts.

use objects::Sha256Digest;
use question_model::{CourseInstanceReceiptTarget, CurriculumAdoptionPreviewRequest, UserId};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    StoreError,
    canonical_json::{CanonicalJsonV1, canonical_json_bytes_v1},
};

const REQUEST_DIGEST_DOMAIN: &str = "ple:curriculum-adoption-request:v1";
const RECONCILIATION_TARGET_DIGEST_DOMAIN: &str =
    "ple:curriculum-adoption-reconciliation-target:v1";
const REQUEST_DIGEST_VERSION: u8 = 1;

/// Closed private receipt operation vocabulary for BlueprintCourse and CourseInstance work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CurriculumAdoptionOperation {
    ForkBlueprintCourse,
    AdoptBlueprintAssignment,
    InstantiateBlueprintCourse,
    RolloverCourseInstance,
    ShiftCourseInstanceTerm,
    ControlledUpdateBlueprintAssignment,
    CreateSelectedBlueprintAssignment,
    ReconcileCourseInstanceAdoption,
}

/// Fixed-width request binding retained by private receipt projections.
pub(crate) type CurriculumAdoptionRequestDigest = Sha256Digest;

#[derive(Serialize)]
struct RequestIntentEnvelope<'a> {
    domain: &'static str,
    version: u8,
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    request: &'a CurriculumAdoptionPreviewRequest,
}

/// One immutable canonical browser intent shared by Memory and PostgreSQL receipt writers.
///
/// The retained `CanonicalJsonV1` source is the sole byte authority. Its
/// projection and digest are read from that one encoding, so later writers
/// preserve the request evidence without serializing the typed request again.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CanonicalCurriculumAdoptionIntentV1 {
    operation: CurriculumAdoptionOperation,
    canonical_json: CanonicalJsonV1,
}

impl CanonicalCurriculumAdoptionIntentV1 {
    /// Encodes one strict browser request once under its server-derived global actor.
    pub(crate) fn new(
        operation: CurriculumAdoptionOperation,
        actor: UserId,
        request: &CurriculumAdoptionPreviewRequest,
    ) -> Result<Self, StoreError> {
        let canonical_json = canonical_json_bytes_v1(
            "curriculum adoption request intent",
            &RequestIntentEnvelope {
                domain: REQUEST_DIGEST_DOMAIN,
                version: REQUEST_DIGEST_VERSION,
                operation,
                actor,
                request,
            },
        )?;
        Ok(Self {
            operation,
            canonical_json,
        })
    }

    /// Returns the closed operation bound into the canonical request source.
    pub(crate) fn operation(&self) -> CurriculumAdoptionOperation {
        self.operation
    }

    /// Returns the retained canonical source, projection, version, and digest.
    pub(crate) fn canonical_json(&self) -> &CanonicalJsonV1 {
        &self.canonical_json
    }

    /// Returns the byte-attested request digest for Memory replay and idempotency checks.
    pub(crate) fn request_digest(&self) -> CurriculumAdoptionRequestDigest {
        self.canonical_json.sha256
    }
}

/// Hashes the server-only immutable target of one reconciliation operation.
pub(crate) fn reconciliation_target_digest(
    actor: UserId,
    target: &CourseInstanceReceiptTarget,
) -> CurriculumAdoptionRequestDigest {
    let mut hasher = Sha256::new();
    hasher.update(RECONCILIATION_TARGET_DIGEST_DOMAIN);
    hasher.update([REQUEST_DIGEST_VERSION]);
    hasher.update(b"reconcile_course_instance_adoption\0");
    hasher.update(actor.as_uuid().as_bytes());
    hasher.update(reconciliation_operation_name(target).as_bytes());
    hasher.update(target.destination().course.number().to_be_bytes());
    hasher.update(target.destination().schedule_revision.value().to_be_bytes());
    hasher.update(target.idempotency_key().as_str().as_bytes());
    hasher.update(target.request_digest());
    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn reconciliation_operation_name(target: &CourseInstanceReceiptTarget) -> &'static str {
    match target.operation() {
        question_model::CourseInstanceOperationKind::Rollover => "rollover",
        question_model::CourseInstanceOperationKind::ShiftTerm => "shift_term",
        question_model::CourseInstanceOperationKind::ControlledUpdate => "controlled_update",
        question_model::CourseInstanceOperationKind::SelectedCopy => "selected_copy",
        question_model::CourseInstanceOperationKind::Reconcile => "reconcile",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        ActivityTimestamp, BlueprintReference, BlueprintRevision, CourseInstanceEligibility,
        CourseInstanceReceiptTarget, CourseInstanceWitness, CourseReference,
        CourseScheduleRevision, CurriculumPinReplacements, ObservedBlueprintSource,
        ShiftCourseInstanceTermApplyRecord, ShiftCourseInstanceTermReceipt,
    };
    use uuid::Uuid;

    fn actor(value: u128) -> UserId {
        UserId::from_uuid(Uuid::from_u128(value))
    }

    fn fork_request(revision: u64) -> CurriculumAdoptionPreviewRequest {
        CurriculumAdoptionPreviewRequest::ForkBlueprintCourse {
            request: question_model::ForkBlueprintCoursePreviewRequest {
                source: ObservedBlueprintSource {
                    reference: BlueprintReference::new(7).expect("blueprint reference"),
                    revision: BlueprintRevision::new(revision).expect("blueprint revision"),
                },
                replacements: CurriculumPinReplacements::default(),
            },
        }
    }

    fn reconciliation_target() -> CourseInstanceReceiptTarget {
        let destination = CourseInstanceWitness::new(
            CourseReference::new(3).expect("course reference"),
            CourseScheduleRevision::new(1).expect("schedule revision"),
            vec![],
        )
        .expect("course instance witness");
        let receipt = ShiftCourseInstanceTermReceipt::from_server_record(
            ShiftCourseInstanceTermApplyRecord::new(
                destination,
                question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-12",
                    "America/Chicago",
                )
                .expect("course term"),
                vec![],
                actor(1),
                [7; 32],
                question_model::CurriculumAdoptionIdempotencyKey::parse("reconcile-target")
                    .expect("idempotency key"),
                CourseInstanceEligibility::Eligible,
            )
            .expect("shift record"),
            ActivityTimestamp::from_unix_millis(1),
        );
        CourseInstanceReceiptTarget::ShiftTerm(receipt)
    }

    #[test]
    fn canonical_intent_retains_exact_binding_and_source_coherence() {
        let request = fork_request(2);
        let intent = CanonicalCurriculumAdoptionIntentV1::new(
            CurriculumAdoptionOperation::ForkBlueprintCourse,
            actor(1),
            &request,
        )
        .expect("canonical intent");
        let canonical = intent.canonical_json();

        assert_eq!(
            intent.operation(),
            CurriculumAdoptionOperation::ForkBlueprintCourse
        );
        assert_eq!(
            canonical.version,
            crate::canonical_json::CANONICAL_JSON_V1_VERSION
        );
        assert_eq!(canonical.projection["domain"], REQUEST_DIGEST_DOMAIN);
        assert_eq!(canonical.projection["version"], REQUEST_DIGEST_VERSION);
        assert_eq!(canonical.projection["operation"], "fork_blueprint_course");
        assert_eq!(
            canonical.projection["actor"],
            serde_json::to_value(actor(1)).unwrap()
        );
        assert_eq!(
            canonical.projection["request"],
            serde_json::to_value(&request).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&canonical.source).unwrap(),
            canonical.projection
        );
        assert_eq!(
            canonical.sha256,
            Sha256Digest::compute(canonical.source.as_bytes())
        );
        assert_eq!(intent.request_digest(), canonical.sha256);
    }

    #[test]
    fn same_intent_is_stable_and_changes_bind_operation_actor_and_request() {
        let request = fork_request(2);
        let baseline = CanonicalCurriculumAdoptionIntentV1::new(
            CurriculumAdoptionOperation::ForkBlueprintCourse,
            actor(1),
            &request,
        )
        .expect("baseline intent");
        let repeat = CanonicalCurriculumAdoptionIntentV1::new(
            CurriculumAdoptionOperation::ForkBlueprintCourse,
            actor(1),
            &request,
        )
        .expect("repeat intent");
        assert_eq!(baseline, repeat);

        assert_ne!(
            baseline.request_digest(),
            CanonicalCurriculumAdoptionIntentV1::new(
                CurriculumAdoptionOperation::AdoptBlueprintAssignment,
                actor(1),
                &request,
            )
            .expect("operation variant")
            .request_digest()
        );
        assert_ne!(
            baseline.request_digest(),
            CanonicalCurriculumAdoptionIntentV1::new(
                CurriculumAdoptionOperation::ForkBlueprintCourse,
                actor(2),
                &request,
            )
            .expect("actor variant")
            .request_digest()
        );
        assert_ne!(
            baseline.request_digest(),
            CanonicalCurriculumAdoptionIntentV1::new(
                CurriculumAdoptionOperation::ForkBlueprintCourse,
                actor(1),
                &fork_request(3),
            )
            .expect("request variant")
            .request_digest()
        );
    }

    #[test]
    fn reconciliation_digest_uses_distinct_server_only_domain() {
        assert_ne!(REQUEST_DIGEST_DOMAIN, RECONCILIATION_TARGET_DIGEST_DOMAIN);
        let target = reconciliation_target();
        let target_digest = reconciliation_target_digest(actor(9), &target);
        let browser_digest = CanonicalCurriculumAdoptionIntentV1::new(
            CurriculumAdoptionOperation::ForkBlueprintCourse,
            actor(9),
            &fork_request(2),
        )
        .expect("browser intent")
        .request_digest();

        assert_ne!(target_digest, browser_digest);
        assert_ne!(
            target_digest,
            Sha256Digest::from_bytes(target.request_digest())
        );
    }
}
