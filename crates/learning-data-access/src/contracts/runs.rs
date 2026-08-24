use super::*;
use question_model::QuestionEnvelope;
use question_model::presentation::AssetBindingV1;

#[path = "runs/issue_contracts.rs"]
mod issue_contracts;
pub use issue_contracts::*;
#[path = "runs/qti_contracts.rs"]
mod qti_contracts;
pub use qti_contracts::*;

/// Server-only course and assignment routing context for learner work.
///
/// This is a routing assertion that the [`Store`] verifies against the
/// authoritative learner-work records; it is not an authority or
/// authorization grant. The trusted server boundary constructs it from the
/// typed route values before calling learner-work persistence operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnerWorkRoutingBinding {
    pub course: CourseId,
    pub assignment: AssignmentId,
}

impl LearnerWorkRoutingBinding {
    /// Constructs a routing assertion for one course and assignment pair.
    pub const fn new(course: CourseId, assignment: AssignmentId) -> Self {
        Self { course, assignment }
    }
}

/// Common immutable, server-only receipt evidence for an issued attempt.
///
/// The Store creates this only after it has established the caller's current
/// learner entitlement for the explicit course/assignment route.  It is
/// intentionally neither serializable nor field-public: browser handlers use
/// only [`Self::presentation_snapshot`], while trusted grading and rehearsal
/// boundaries take the specific evidence they require.  Future issued-question
/// snapshot revisions extend this coherent capability rather than reintroduce
/// a catalog read at receipt time.
#[derive(Clone, PartialEq)]
pub struct IssuedAttemptReceiptEvidence {
    presentation_binding: Option<PresentationBindingV1>,
    presentation_snapshot: Option<ReceiptPresentationSnapshot>,
    grading_envelope: Option<QuestionEnvelope>,
}

impl IssuedAttemptReceiptEvidence {
    pub(crate) fn new(
        presentation_binding: Option<PresentationBindingV1>,
        presentation_snapshot: Option<ReceiptPresentationSnapshot>,
        grading_envelope: Option<QuestionEnvelope>,
    ) -> Self {
        Self {
            presentation_binding,
            presentation_snapshot,
            grading_envelope,
        }
    }

    /// Returns the immutable capability binding for trusted receipt checks.
    pub fn presentation_binding(&self) -> Option<PresentationBindingV1> {
        self.presentation_binding
    }

    /// Returns the answer-free learner-facing snapshot, when this attempt
    /// issued a presentation.
    pub fn presentation_snapshot(&self) -> Option<&ReceiptPresentationSnapshot> {
        self.presentation_snapshot.as_ref()
    }

    /// Returns the retained server-only envelope for trusted grading checks.
    pub fn grading_envelope(&self) -> Option<&QuestionEnvelope> {
        self.grading_envelope.as_ref()
    }
}

impl std::fmt::Debug for IssuedAttemptReceiptEvidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedAttemptReceiptEvidence")
            .field("presentation_binding", &"[SERVER-ONLY]")
            .field("presentation_snapshot", &"[ANSWER-FREE SERVER-ONLY]")
            .field("grading_envelope", &"[SERVER-ONLY]")
            .finish()
    }
}

/// Active-only private grading evidence.  This is absent after submission so
/// a successful delivery can never depend on deleted replay authority.
#[derive(Clone, PartialEq)]
pub struct ActiveIssuedAttemptEvidence {
    receipt: IssuedAttemptReceiptEvidence,
    flat_grading: Option<crate::IssuedFlatGradingContract>,
    webwork_grading: Option<IssuedWebworkGradingContract>,
    webwork_replay: Option<WebworkGradeReplayStateV1>,
}

impl ActiveIssuedAttemptEvidence {
    pub(crate) fn new(
        receipt: IssuedAttemptReceiptEvidence,
        flat_grading: Option<crate::IssuedFlatGradingContract>,
        webwork_grading: Option<IssuedWebworkGradingContract>,
        webwork_replay: Option<WebworkGradeReplayStateV1>,
    ) -> Self {
        Self {
            receipt,
            flat_grading,
            webwork_grading,
            webwork_replay,
        }
    }

    pub fn receipt_evidence(&self) -> &IssuedAttemptReceiptEvidence {
        &self.receipt
    }

    pub fn presentation_snapshot(&self) -> Option<&ReceiptPresentationSnapshot> {
        self.receipt.presentation_snapshot()
    }

    pub fn flat_grading(&self) -> Option<&crate::IssuedFlatGradingContract> {
        self.flat_grading.as_ref()
    }

    pub fn webwork_grading(&self) -> Option<&IssuedWebworkGradingContract> {
        self.webwork_grading.as_ref()
    }

    pub fn webwork_replay(&self) -> Option<&WebworkGradeReplayStateV1> {
        self.webwork_replay.as_ref()
    }
}

impl std::fmt::Debug for ActiveIssuedAttemptEvidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ActiveIssuedAttemptEvidence")
            .field("receipt", &"[SERVER-ONLY]")
            .field("flat_grading", &"[SERVER-ONLY]")
            .field("webwork_grading", &"[SERVER-ONLY]")
            .field("webwork_replay", &"[SERVER-ONLY]")
            .finish()
    }
}

/// Minimal immutable receipt projection for submitted question delivery.
#[derive(Clone, PartialEq)]
pub struct SubmittedQuestionReceipt {
    presentation: Option<ReceiptPresentationSnapshot>,
}

/// Submitted lifecycle evidence with a checked immutable answer-free receipt.
#[derive(Clone, PartialEq)]
pub struct SubmittedIssuedAttemptRead {
    evidence: IssuedAttemptReceiptEvidence,
    receipt: SubmittedQuestionReceipt,
}

impl SubmittedIssuedAttemptRead {
    pub(crate) fn new(
        evidence: IssuedAttemptReceiptEvidence,
        receipt: SubmittedQuestionReceipt,
    ) -> Self {
        Self { evidence, receipt }
    }

    pub fn receipt_evidence(&self) -> &IssuedAttemptReceiptEvidence {
        &self.evidence
    }

    pub fn presentation(&self) -> Option<&ReceiptPresentationSnapshot> {
        self.receipt.presentation()
    }
}

impl std::fmt::Debug for SubmittedIssuedAttemptRead {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SubmittedIssuedAttemptRead([SERVER-ONLY])")
    }
}

/// Terminal lifecycle evidence for an attempt with no learner delivery receipt.
#[derive(Clone, PartialEq)]
pub struct TerminalIssuedAttemptRead {
    evidence: IssuedAttemptReceiptEvidence,
    status: AttemptStatus,
}

impl TerminalIssuedAttemptRead {
    pub(crate) fn new(evidence: IssuedAttemptReceiptEvidence, status: AttemptStatus) -> Self {
        Self { evidence, status }
    }

    pub fn receipt_evidence(&self) -> &IssuedAttemptReceiptEvidence {
        &self.evidence
    }

    pub fn status(&self) -> AttemptStatus {
        self.status
    }
}

impl std::fmt::Debug for TerminalIssuedAttemptRead {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TerminalIssuedAttemptRead")
            .field("status", &self.status)
            .field("evidence", &"[SERVER-ONLY]")
            .finish()
    }
}

impl SubmittedQuestionReceipt {
    pub(crate) fn new(presentation: Option<ReceiptPresentationSnapshot>) -> Self {
        Self { presentation }
    }

    pub fn presentation(&self) -> Option<&ReceiptPresentationSnapshot> {
        self.presentation.as_ref()
    }
}

impl std::fmt::Debug for SubmittedQuestionReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubmittedQuestionReceipt")
            .field("presentation", &"[ANSWER-FREE SERVER-ONLY]")
            .finish()
    }
}

/// Authorized current question-delivery lifecycle plus immutable issuance
/// evidence. This is the exclusive broker-first question-delivery read.
#[derive(Clone, PartialEq)]
pub enum IssuedAttemptRead {
    Active(Box<ActiveIssuedAttemptEvidence>),
    Submitted(Box<SubmittedIssuedAttemptRead>),
    TerminalWithoutReceipt(Box<TerminalIssuedAttemptRead>),
}

impl IssuedAttemptRead {
    pub fn receipt_evidence(&self) -> &IssuedAttemptReceiptEvidence {
        match self {
            Self::Active(evidence) => evidence.receipt_evidence(),
            Self::Submitted(read) => read.receipt_evidence(),
            Self::TerminalWithoutReceipt(read) => read.receipt_evidence(),
        }
    }

    pub fn active_presentation_snapshot(&self) -> Option<&ReceiptPresentationSnapshot> {
        match self {
            Self::Active(evidence) => evidence.presentation_snapshot(),
            Self::Submitted(_) | Self::TerminalWithoutReceipt(_) => None,
        }
    }

    pub fn submitted_presentation(&self) -> Option<&ReceiptPresentationSnapshot> {
        match self {
            Self::Submitted(read) => read.presentation(),
            Self::Active(_) | Self::TerminalWithoutReceipt(_) => None,
        }
    }

    pub fn terminal_status(&self) -> Option<AttemptStatus> {
        match self {
            Self::TerminalWithoutReceipt(read) => Some(read.status()),
            Self::Active(_) | Self::Submitted(_) => None,
        }
    }
}

impl std::fmt::Debug for IssuedAttemptRead {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active(_) => formatter.write_str("IssuedAttemptRead::Active([SERVER-ONLY])"),
            Self::Submitted(_) => {
                formatter.write_str("IssuedAttemptRead::Submitted([SERVER-ONLY])")
            }
            Self::TerminalWithoutReceipt(read) => formatter
                .debug_tuple("IssuedAttemptRead::TerminalWithoutReceipt")
                .field(read)
                .finish(),
        }
    }
}

/// One private upstream field/value pair for a rendered selectable item.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebworkReplayControlV1 {
    pub item: question_model::presentation::RenderedItemIdV1,
    pub field: String,
    pub value: String,
}

/// One private upstream matching field and its visible choice-value map.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebworkReplayMatchPromptV1 {
    pub prompt: question_model::presentation::RenderedItemIdV1,
    pub field: String,
    pub choices: Vec<WebworkReplayControlV1>,
}

/// Answer-free WeBWorK form replay state bound to one issued presentation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WebworkReplayMappingV1 {
    SingleChoice {
        items: Vec<WebworkReplayControlV1>,
    },
    Matching {
        items: Vec<WebworkReplayMatchPromptV1>,
    },
}

impl std::fmt::Debug for WebworkReplayMappingV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, items) = match self {
            Self::SingleChoice { items } => ("singleChoice", items.len()),
            Self::Matching { items } => ("matching", items.len()),
        };
        formatter
            .debug_struct("WebworkReplayMappingV1")
            .field("kind", &kind)
            .field("items", &items)
            .field("mapping", &"[REDACTED]")
            .finish()
    }
}

impl WebworkReplayMappingV1 {
    pub(crate) fn validate(&self) -> Result<(), StoreError> {
        fn valid_field(value: &str) -> bool {
            value.len() <= 128
                && value.starts_with("AnSwEr")
                && !value[6..].is_empty()
                && value[6..].bytes().all(|byte| byte.is_ascii_digit())
        }
        fn valid_value(value: &str) -> bool {
            !value.is_empty() && value.len() <= 512 && !value.contains('\0')
        }
        match self {
            Self::SingleChoice { items } => {
                if !(2..=32).contains(&items.len()) {
                    return Err(StoreError::InvalidRecord(
                        "WeBWorK replay choice count is outside the supported bound".into(),
                    ));
                }
                let mut item_ids = std::collections::BTreeSet::new();
                let mut values = std::collections::BTreeSet::new();
                let mut field = None;
                for item in items {
                    if !item_ids.insert(item.item.clone())
                        || !values.insert(item.value.clone())
                        || !valid_field(&item.field)
                        || !valid_value(&item.value)
                        || *field.get_or_insert(&item.field) != &item.field
                    {
                        return Err(StoreError::InvalidRecord(
                            "WeBWorK replay choice mapping is invalid".into(),
                        ));
                    }
                }
            }
            Self::Matching { items } => {
                if !(2..=26).contains(&items.len()) {
                    return Err(StoreError::InvalidRecord(
                        "WeBWorK replay matching count is outside the supported bound".into(),
                    ));
                }
                let mut prompts = std::collections::BTreeSet::new();
                let mut fields = std::collections::BTreeSet::new();
                let mut expected_choices = None;
                for item in items {
                    if !prompts.insert(item.prompt.clone())
                        || !fields.insert(item.field.clone())
                        || !valid_field(&item.field)
                        || item.choices.len() != items.len()
                    {
                        return Err(StoreError::InvalidRecord(
                            "WeBWorK replay matching prompt is invalid".into(),
                        ));
                    }
                    let mut choice_ids = std::collections::BTreeSet::new();
                    let mut values = std::collections::BTreeSet::new();
                    for choice in &item.choices {
                        if !choice_ids.insert(choice.item.clone())
                            || !values.insert(choice.value.clone())
                            || choice.field != item.field
                            || !valid_value(&choice.value)
                        {
                            return Err(StoreError::InvalidRecord(
                                "WeBWorK replay matching choice is invalid".into(),
                            ));
                        }
                    }
                    if expected_choices
                        .as_ref()
                        .is_some_and(|expected| expected != &choice_ids)
                    {
                        return Err(StoreError::InvalidRecord(
                            "WeBWorK replay matching choice sets disagree".into(),
                        ));
                    }
                    expected_choices.get_or_insert(choice_ids);
                }
            }
        }
        Ok(())
    }
}

/// Exact persisted metadata used to validate one private grade replay.
#[derive(Clone, PartialEq, Eq)]
pub struct WebworkGradeReplayStateV1 {
    pub problem: ProblemId,
    pub version: VersionId,
    pub source_artifact: question_model::SourceArtifact,
    pub seed: u64,
    pub renderer: question_model::ImplementationVersion,
    pub presentation_digest: question_model::PresentationDigestV1,
    pub mapping: WebworkReplayMappingV1,
}

impl std::fmt::Debug for WebworkGradeReplayStateV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebworkGradeReplayStateV1")
            .field("problem", &self.problem)
            .field("version", &self.version)
            .field("source_artifact", &self.source_artifact)
            .field("seed", &self.seed)
            .field("renderer", &self.renderer)
            .field("presentation_digest", &self.presentation_digest)
            .field("mapping", &"[REDACTED]")
            .finish()
    }
}

/// Exact server-only definition used by a first WeBWorK grade.
///
/// The definition has no answer key, but it fixes the source path, grading
/// policy, immutable problem/version identity, and capability profile without
/// a current catalog read. The matching source bytes stay separately bound by
/// [`WebworkGradeReplayStateV1::source_artifact`].
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct IssuedWebworkGradingContract {
    question: question_model::QuestionDefinition,
}

impl IssuedWebworkGradingContract {
    /// Retains one exact issued WebWork definition.
    pub fn new(question: question_model::QuestionDefinition) -> Result<Self, StoreError> {
        if !matches!(
            question.source,
            question_model::QuestionSource::Webwork { .. }
        ) {
            return Err(StoreError::InvalidRecord(
                "WebWork grading contract requires a WebWork question".to_string(),
            ));
        }
        Ok(Self { question })
    }

    /// The immutable definition consumed only by trusted grading code.
    pub fn question(&self) -> &question_model::QuestionDefinition {
        &self.question
    }

    pub(crate) fn validate_for_attempt(&self, attempt: &QuestionAttempt) -> Result<(), StoreError> {
        if !matches!(
            self.question.source,
            question_model::QuestionSource::Webwork { .. }
        ) || self.question.problem != attempt.problem
            || self.question.version != attempt.question_version
        {
            return Err(StoreError::Unavailable(
                "stored WebWork grading contract disagrees with its attempt".to_string(),
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for IssuedWebworkGradingContract {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedWebworkGradingContract")
            .field("question", &"[SERVER-ONLY]")
            .finish()
    }
}

pub(crate) fn webwork_replay_state_from_issue(
    problem: ProblemId,
    version: VersionId,
    seed: u64,
    provenance: &AttemptProvenance,
    presentation: PresentationBindingV1,
    mapping: WebworkReplayMappingV1,
) -> Result<WebworkGradeReplayStateV1, StoreError> {
    mapping.validate()?;
    let source_artifact = provenance.source_artifact.clone().ok_or_else(|| {
        StoreError::InvalidRecord("WeBWorK replay lacks immutable source provenance".into())
    })?;
    let renderer = provenance.renderer.clone().ok_or_else(|| {
        StoreError::InvalidRecord("WeBWorK replay lacks renderer provenance".into())
    })?;
    Ok(WebworkGradeReplayStateV1 {
        problem,
        version,
        source_artifact,
        seed,
        renderer,
        presentation_digest: presentation.digest(),
        mapping,
    })
}

/// Validates that persisted private replay state still belongs to the exact
/// attempt presentation that owns it.
///
/// Storage calls this after authorization and before returning replay state to
/// a grader. A row that is individually well formed but cross-wired to another
/// attempt is unavailable authority, never a usable mapping.
pub(crate) fn validate_persisted_webwork_replay_state(
    attempt: &QuestionAttempt,
    presentation: Option<PresentationBindingV1>,
    state: &WebworkGradeReplayStateV1,
) -> Result<(), StoreError> {
    state
        .mapping
        .validate()
        .map_err(|_| StoreError::Unavailable("stored WeBWorK replay mapping is invalid".into()))?;
    let Some(presentation) = presentation else {
        return Err(StoreError::Unavailable(
            "stored WeBWorK replay lacks its presentation binding".into(),
        ));
    };
    if state.problem != attempt.problem
        || state.version != attempt.question_version
        || state.seed != attempt.seed
        || attempt.provenance.source_artifact.as_ref() != Some(&state.source_artifact)
        || attempt.provenance.renderer.as_ref() != Some(&state.renderer)
        || state.presentation_digest != presentation.digest()
    {
        return Err(StoreError::Unavailable(
            "stored WeBWorK replay disagrees with its owning attempt".into(),
        ));
    }
    Ok(())
}

/// Bounded client-generated key for replaying one submission safely.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubmissionIdempotencyKey(String);

impl SubmissionIdempotencyKey {
    /// Parses one visible ASCII key without accepting whitespace or controls.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        const MAX_KEY_BYTES: usize = 200;
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_KEY_BYTES
            || !value.is_ascii()
            || value
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err(StoreError::InvalidRecord(
                "idempotency key must contain 1 to 200 visible ASCII characters".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated header value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Issue, prefetch, and prepared-submission contracts live in the sibling
// issue-contract module so this shared lifecycle module stays focused.

/// Validates the immutable issued-presentation tuple before an attempt is
/// written. The same validation is repeated while loading a receipt so a
/// corrupt persisted payload fails closed rather than becoming a new view.
pub(crate) fn validate_issued_presentation(
    capability: PresentationCapability,
    attempt: &QuestionAttempt,
    binding: Option<PresentationBindingV1>,
    snapshot: Option<&ReceiptPresentationSnapshot>,
    grading_envelope: Option<&QuestionEnvelope>,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    use question_model::IssuedAttemptCapabilityV1 as Capability;

    let presentation_matches_attempt = matches!(
        (attempt.issued_capability, capability),
        (
            Capability::PresentationEnvelope
                | Capability::FlatPresentation
                | Capability::WebworkPresentation
                | Capability::QtiPresentation,
            PresentationCapability::EnvelopeV1,
        ) | (
            Capability::NotApplicable,
            PresentationCapability::NotApplicable
        )
    );
    if !presentation_matches_attempt {
        return Err(StoreError::Unavailable(
            "stored presentation capability disagrees with its checksummed attempt".to_string(),
        ));
    }
    match (capability, binding, snapshot, grading_envelope) {
        (PresentationCapability::NotApplicable, None, None, None) => Ok(None),
        (PresentationCapability::NotApplicable, _, _, _) => Err(StoreError::InvalidRecord(
            "a non-presentation attempt carries presentation state".to_string(),
        )),
        (
            PresentationCapability::EnvelopeV1,
            Some(binding),
            Some(snapshot),
            Some(grading_envelope),
        ) => {
            if snapshot.envelope.version != attempt.question_version
                || snapshot.envelope.seed.value() != attempt.seed
                || grading_envelope.version != attempt.question_version
                || grading_envelope.seed.value() != attempt.seed
            {
                return Err(StoreError::InvalidRecord(
                    "issued presentation does not match its attempt".to_string(),
                ));
            }
            let rebuilt = question_model::presentation::reproduce_presentation_v1(
                grading_envelope,
                &snapshot.asset_bindings,
                binding,
            )
            .map_err(|error| {
                StoreError::InvalidRecord(format!("issued presentation is invalid: {error}"))
            })?;
            if rebuilt.envelope != snapshot.envelope || rebuilt.asset_bindings != snapshot.asset_bindings {
                return Err(StoreError::InvalidRecord(
                    "issued presentation does not match its private grading contract".to_string(),
                ));
            }
            Ok(Some(snapshot.clone()))
        }
        (PresentationCapability::EnvelopeV1, _, _, _) => Err(StoreError::Unavailable(
            "a presentation-bearing attempt lacks its immutable snapshot, binding, or grading contract"
                .to_string(),
        )),
    }
}

/// Validates flat private grading authority while the exact published version
/// is still available at issuance. Later first-submit code reads only this
/// contract and its issued presentation tuple.
pub(crate) fn validate_issued_flat_grading(
    question: &question_model::QuestionDefinition,
    capability: PresentationCapability,
    flat_capability: FlatGradingCapability,
    contract: Option<&crate::IssuedFlatGradingContract>,
) -> Result<(), StoreError> {
    let is_flat = matches!(
        &question.source,
        question_model::QuestionSource::Native { family }
            if grading::flat_question::is_flat_question_family(family)
    );
    match (is_flat, capability, flat_capability, contract) {
        (
            true,
            PresentationCapability::EnvelopeV1,
            FlatGradingCapability::Required,
            Some(contract),
        ) if contract.question() == question => Ok(()),
        (true, _, _, _) => Err(StoreError::InvalidRecord(
            "flat-question issuance lacks its immutable private grading contract".to_string(),
        )),
        (false, _, FlatGradingCapability::NotApplicable, None) => Ok(()),
        (false, _, _, _) => Err(StoreError::InvalidRecord(
            "non-flat issuance carries private flat-question grading authority".to_string(),
        )),
    }
}

/// Validates the explicit first-grade WebWork authority while the exact
/// published version is available at issuance. Later submission reads only
/// this frozen contract and the attempt-bound source artifact.
pub(crate) fn validate_issued_webwork_grading(
    question: &question_model::QuestionDefinition,
    capability: WebworkGradingCapability,
    contract: Option<&IssuedWebworkGradingContract>,
) -> Result<(), StoreError> {
    let is_webwork = matches!(
        question.source,
        question_model::QuestionSource::Webwork { .. }
    );
    match (is_webwork, capability, contract) {
        (true, WebworkGradingCapability::Required, Some(contract))
            if contract.question() == question =>
        {
            Ok(())
        }
        (true, _, _) => Err(StoreError::InvalidRecord(
            "WeBWorK issuance lacks its immutable private grading contract".to_string(),
        )),
        (false, WebworkGradingCapability::NotApplicable, None) => Ok(()),
        (false, _, _) => Err(StoreError::InvalidRecord(
            "non-WeBWorK issuance carries private WeBWorK grading authority".to_string(),
        )),
    }
}

/// Validates the replay controls required to translate a first WeBWorK
/// submission without reopening the current source or renderer.
///
/// `validate_issued_webwork_grading` establishes which source family owns the
/// contract. This companion check makes the required private control mapping
/// equally explicit, rather than treating its nullable storage as a legacy
/// recovery branch.
pub(crate) fn validate_issued_webwork_replay(
    capability: WebworkGradingCapability,
    mapping: Option<&WebworkReplayMappingV1>,
) -> Result<(), StoreError> {
    match (capability, mapping) {
        (WebworkGradingCapability::Required, Some(mapping)) => mapping.validate(),
        (WebworkGradingCapability::Required, None) => Err(StoreError::InvalidRecord(
            "WeBWorK issuance lacks its immutable replay mapping".to_string(),
        )),
        (WebworkGradingCapability::NotApplicable, None) => Ok(()),
        (WebworkGradingCapability::NotApplicable, Some(_)) => Err(StoreError::InvalidRecord(
            "non-WeBWorK issuance carries a replay mapping".to_string(),
        )),
    }
}

/// Exact answer-free descriptor inputs retained with one immutable receipt.
///
/// `PresentationEnvelopeV1` names visible content, but the descriptor also
/// hashes the selected public asset renditions. Retaining both is therefore
/// necessary to reproduce and validate any asset-backed response (including a
/// hotspot) without consulting mutable catalog state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptPresentationSnapshot {
    pub envelope: PresentationEnvelopeV1,
    pub asset_bindings: Vec<AssetBindingV1>,
}

impl std::fmt::Debug for SubmitQuestionAttemptCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitQuestionAttemptCommand")
            .field("actor", &self.actor)
            .field("binding", &self.binding)
            .field("attempt", &self.attempt)
            .field("response", &self.response)
            .field("result", &self.result)
            .field("idempotency_key", &self.idempotency_key)
            .field("feedback", &"[redacted]")
            .finish()
    }
}

/// Stable idempotency and audit identity for one instructor support action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptSupportActionId(Uuid);

impl AttemptSupportActionId {
    /// Wraps an identity read from storage or a trusted server boundary.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID persisted with the audit event.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Mints one server-owned action identity.
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!(
                "attempt support action ID randomness unavailable: {error}"
            ))
        })
        .map(Self)
    }
}

/// Closed set of sensitive attempt-support mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptSupportAction {
    /// Close an active question without fabricating a response or grade.
    ForceSubmit,
    /// Exclude an attempt from current scoring while retaining its evidence.
    Clear,
}

impl AttemptSupportAction {
    #[cfg(feature = "postgres")]
    pub(crate) fn audit_name(self) -> &'static str {
        match self {
            Self::ForceSubmit => "attempt.force_submit",
            Self::Clear => "attempt.clear",
        }
    }
}

/// Idempotent instructor request to close one still-active question attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceSubmitAttemptCommand {
    pub action: AttemptSupportActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
}

/// Idempotent instructor request to remove one attempt from current scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearAttemptCommand {
    pub action: AttemptSupportActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
}

/// Minimal retained evidence for one instructor attempt-support action.
///
/// No response, evaluation, score, student identity, or obsolete grade is
/// copied into this record. The protected attempt remains the evidence owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptSupportRecord {
    pub tenant: TenantId,
    pub action: AttemptSupportActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub kind: AttemptSupportAction,
    pub previous_status: AttemptStatus,
    pub resulting_status: AttemptStatus,
    pub occurred_at: ActivityTimestamp,
}

/// First committed submission result or an exact idempotent replay of it.
#[derive(Clone, PartialEq)]
pub struct SubmissionRecord {
    /// Browser-safe attempt projection with response and disclosed result data.
    pub attempt: QuestionAttempt,
    /// Run after any completion derived by this submission.
    pub run: AssignmentRun,
    /// Compact projection updated in the same transaction as the submission.
    pub summary: StudentAssignmentSummary,
    /// Private, immutable teaching content retained for policy-controlled
    /// disclosure. This is intentionally not browser-safe data.
    pub feedback: AttemptFeedbackRecord,
    /// Answer-free envelope actually rendered for this receipt, when the
    /// response family has a native presentation.
    pub presentation: Option<ReceiptPresentationSnapshot>,
    /// Current server-side disclosure input for this receipt's projection.
    /// Immutable grading and feedback content remain in the receipt; a later
    /// assignment policy or authoritative-clock change may alter only this
    /// public projection.
    pub disclosure: LearnerDisclosureInput,
}

impl std::fmt::Debug for SubmissionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmissionRecord")
            .field("attempt", &self.attempt)
            .field("run", &self.run)
            .field("summary", &self.summary)
            .field("feedback", &"[redacted]")
            .field(
                "presentation",
                &self.presentation.as_ref().map(|_| "[answer-free]"),
            )
            .field("disclosure", &"[SERVER-ONLY]")
            .finish()
    }
}

/// One atomic activity write applied with its compact summary projection.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityTransition {
    /// Creates a new incomplete run and records its start activity.
    StartRun {
        /// New run with server-supplied start time.
        run: AssignmentRun,
    },
    /// Appends one immutable question-attempt record.
    RecordQuestionAttempt {
        /// Attempt carrying response, result, timing, and reproducibility data.
        attempt: Box<QuestionAttempt>,
    },
    /// Completes an existing run and projects its score.
    CompleteRun {
        /// Existing run to complete.
        run: RunId,
        /// Final score fraction.
        score: f64,
        /// Authoritative PostgreSQL timestamp.
        at: ActivityTimestamp,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_attempt_evidence_debug_redacts_every_evidence_payload() {
        let rendered = format!("{:?}", IssuedAttemptReceiptEvidence::new(None, None, None));
        assert!(rendered.contains("[ANSWER-FREE SERVER-ONLY]"));
        assert!(rendered.contains("[SERVER-ONLY]"));
        assert!(!rendered.contains("QuestionEnvelope"));
        assert!(!rendered.contains("Webwork"));
    }
}
