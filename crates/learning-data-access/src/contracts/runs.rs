use super::*;

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

/// Server-owned data needed to issue or resume one question instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueQuestionAttemptCommand {
    /// Authenticated enrollment owner.
    pub actor: UserId,
    /// Fresh proposed identity; ignored when an active instance already exists.
    pub attempt: QuestionAttemptId,
    /// Active run receiving the question.
    pub run: RunId,
    /// Zero-based logical assignment position.
    pub assignment_position: u32,
    /// Exact immutable content identity at that position.
    pub problem: ProblemId,
    /// Exact immutable version at that position.
    pub question_version: VersionId,
    /// Fresh operating-system-random seed for the proposed instance.
    pub seed: u64,
    /// Hash of the generated parameters.
    pub parameter_hash: String,
    /// Adapter, generator, renderer, source, asset, and grading provenance.
    pub provenance: AttemptProvenance,
    /// Exact answer-free presentation state for compact-payload families.
    ///
    /// File upload and external-tool responses remain outside presentation v1
    /// until their dedicated transfer or broker contracts ship.
    pub presentation: Option<PresentationBindingV1>,
    /// Private answer-free upstream mapping, present only for WeBWorK.
    pub webwork_replay: Option<WebworkReplayMappingV1>,
    /// Server-owned candidate prepared while the preceding attempt was active.
    /// It is verified and consumed atomically with issuance; browser input can
    /// never create this internal command.
    pub prefetched: Option<PrefetchedQuestion>,
    /// Committed predecessor whose immutable receipt is finalized by this
    /// issuance. This link is written in the same transaction as the attempt.
    pub predecessor_submission: Option<QuestionAttemptId>,
}

/// Immutable successor state for one committed submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionNextAttempt {
    /// Finalization has not yet run (for crash healing of older writes).
    Pending,
    /// This submission completed or exhausted the run without another attempt.
    None,
    /// Exact next attempt issued from this submission.
    Issued(QuestionAttemptId),
}

/// Key-free, tenant-owned preparation for a possible next question.
///
/// This intentionally has neither an attempt identity nor a timer. It cannot
/// receive a response, grade, or summary transition; only matching post-submit
/// issuance may consume it into a real [`QuestionAttempt`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchedQuestion {
    pub tenant: TenantId,
    pub run: RunId,
    pub predecessor: QuestionAttemptId,
    pub assignment_position: u32,
    pub problem: ProblemId,
    pub question_version: VersionId,
    pub seed: u64,
    pub parameter_hash: String,
    pub provenance: AttemptProvenance,
    pub presentation: PresentationBindingV1,
    /// Private answer-free upstream mapping retained for atomic promotion.
    pub webwork_replay: Option<WebworkReplayMappingV1>,
}

/// Trusted server request to create or resume a prefetch reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservePrefetchedQuestionCommand {
    pub actor: UserId,
    pub reservation: PrefetchedQuestion,
}

/// Trusted server result to persist for one student response.
#[derive(Clone, PartialEq)]
pub struct SubmitQuestionAttemptCommand {
    /// Authenticated enrollment owner.
    pub actor: UserId,
    /// Issued question being answered.
    pub attempt: QuestionAttemptId,
    /// Student-controlled response already validated and server-graded.
    pub response: StudentResponse,
    /// Key-free grading result produced inside the server boundary.
    pub result: AttemptResult,
    /// Trusted, sanitized teaching material captured with the first grade.
    ///
    /// This remains server-only: it is not a response DTO and is never
    /// serialized by the public model generator.
    pub feedback: FeedbackContent,
    /// Stable key reused by browser retries of this exact response.
    pub idempotency_key: SubmissionIdempotencyKey,
}

impl std::fmt::Debug for SubmitQuestionAttemptCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitQuestionAttemptCommand")
            .field("actor", &self.actor)
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
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!(
                "attempt support action ID randomness unavailable: {error}"
            ))
        })?;
        Ok(Self(Uuid::from_bytes(bytes)))
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
}

impl std::fmt::Debug for SubmissionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmissionRecord")
            .field("attempt", &self.attempt)
            .field("run", &self.run)
            .field("summary", &self.summary)
            .field("feedback", &"[redacted]")
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
