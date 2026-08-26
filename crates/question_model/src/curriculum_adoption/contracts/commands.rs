//! Server-owned B2 commands that can only be created from matching previews.

use super::{
    AlphaInstantiationPreviewView, AssignmentDefinitionSourceView, AssignmentFastForwardDecision,
    AssignmentFastForwardPreviewView, BlueprintInstantiationPreviewView, CourseRolloverPreviewView,
    CourseScheduleWitness, CourseTermShiftPreviewOutcome, CurriculumAdoptionIdempotencyKey,
    CurriculumAdoptionTitle, CurriculumImportRevision, CurriculumPinReplacements,
    ForkAlphaPreviewView, ObservedAlphaSource, ObservedAssignmentRevision, ObservedBlueprintSource,
    SourceDerivedAssignmentPreviewView,
};
use crate::{CourseReference, CourseTerm};

/// Server-owned Alpha fork command. Source resolution and authority are Store responsibilities.
#[derive(Debug, Clone, PartialEq)]
pub struct ForkAlphaCommand {
    source: ObservedAlphaSource,
    replacements: CurriculumPinReplacements,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ForkAlphaCommand {
    /// Binds apply to the exact source returned by preview.
    pub fn from_preview(
        preview: &ForkAlphaPreviewView,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        if preview.pin_correction.is_some() {
            return Err(CurriculumAdoptionCommandError::CorrectionsRequired);
        }
        Ok(Self {
            source: preview.source,
            replacements: preview.replacements.clone(),
            idempotency_key,
        })
    }

    /// Returns the revision-bound public Alpha source.
    pub fn source(&self) -> ObservedAlphaSource {
        self.source
    }

    /// Returns the exact previewed public-question substitutions.
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// Server-owned Blueprint-to-assignment command.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintInstantiationCommand {
    source: ObservedBlueprintSource,
    course: CourseReference,
    target_term: CourseTerm,
    preview_witness: CourseScheduleWitness,
    replacements: CurriculumPinReplacements,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl BlueprintInstantiationCommand {
    /// Binds apply to the exact source, destination, term, and witness returned by preview.
    pub fn from_preview(
        preview: &BlueprintInstantiationPreviewView,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        require_corrected_preview(&preview.corrections, preview.pin_correction.as_ref())?;
        Ok(Self {
            source: preview.source,
            course: preview.course,
            target_term: preview.target_term.clone(),
            preview_witness: preview.witness.clone(),
            replacements: preview.replacements.clone(),
            idempotency_key,
        })
    }

    /// Returns the revision-bound owner-scoped Blueprint source.
    pub fn source(&self) -> ObservedBlueprintSource {
        self.source
    }

    /// Returns the previewed existing teaching-course destination.
    pub fn course(&self) -> CourseReference {
        self.course
    }

    /// Returns the previewed target term.
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }

    /// Returns the course and assignment revision witness from preview.
    pub fn preview_witness(&self) -> &CourseScheduleWitness {
        &self.preview_witness
    }

    /// Returns the exact previewed public-question substitutions.
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// Server-owned Alpha-to-course command.
#[derive(Debug, Clone, PartialEq)]
pub struct AlphaInstantiationCommand {
    source: ObservedAlphaSource,
    title: CurriculumAdoptionTitle,
    target_term: CourseTerm,
    replacements: CurriculumPinReplacements,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl AlphaInstantiationCommand {
    /// Binds apply to the exact source, title, and term returned by preview.
    pub fn from_preview(
        preview: &AlphaInstantiationPreviewView,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        require_corrected_preview(&preview.corrections, preview.pin_correction.as_ref())?;
        Ok(Self {
            source: preview.source,
            title: preview.course.title.clone(),
            target_term: preview.target_term.clone(),
            replacements: preview.replacements.clone(),
            idempotency_key,
        })
    }

    /// Returns the revision-bound public Alpha source.
    pub fn source(&self) -> ObservedAlphaSource {
        self.source
    }

    /// Returns the previewed destination-course title.
    pub fn title(&self) -> &CurriculumAdoptionTitle {
        &self.title
    }

    /// Returns the previewed target term.
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }

    /// Returns the exact previewed public-question substitutions.
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// Server-owned rollover command. The Store keeps learner state out of the destination.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseRolloverCommand {
    preview_witness: CourseScheduleWitness,
    title: CurriculumAdoptionTitle,
    target_term: CourseTerm,
    replacements: CurriculumPinReplacements,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl CourseRolloverCommand {
    /// Binds apply to the exact witness, title, and term returned by preview.
    pub fn from_preview(
        preview: &CourseRolloverPreviewView,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        require_corrected_preview(&preview.corrections, preview.pin_correction.as_ref())?;
        Ok(Self {
            preview_witness: preview.witness.clone(),
            title: preview.course.title.clone(),
            target_term: preview.target_term.clone(),
            replacements: preview.replacements.clone(),
            idempotency_key,
        })
    }

    /// Returns the source-course revision witness from preview.
    pub fn preview_witness(&self) -> &CourseScheduleWitness {
        &self.preview_witness
    }

    /// Returns the previewed destination-course title.
    pub fn title(&self) -> &CurriculumAdoptionTitle {
        &self.title
    }

    /// Returns the previewed target term.
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }

    /// Returns the exact previewed public-question substitutions.
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// Server-owned atomic term-shift command for one unissued teaching course.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseTermShiftCommand {
    preview_witness: CourseScheduleWitness,
    target_term: CourseTerm,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl CourseTermShiftCommand {
    /// Binds apply to the exact witness and target term returned by preview.
    pub fn from_preview(
        outcome: &CourseTermShiftPreviewOutcome,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        let CourseTermShiftPreviewOutcome::Eligible { preview } = outcome else {
            return Err(CurriculumAdoptionCommandError::TermShiftNotEligible);
        };
        require_corrected_preview(&preview.corrections, None)?;
        Ok(Self {
            preview_witness: preview.witness.clone(),
            target_term: preview.target_term.clone(),
            idempotency_key,
        })
    }

    /// Returns the course and assignment revision witness from preview.
    pub fn preview_witness(&self) -> &CourseScheduleWitness {
        &self.preview_witness
    }

    /// Returns the previewed target term.
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// Server-owned fast-forward command with all required optimistic witnesses.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentFastForwardCommand {
    course: CourseReference,
    assignment: ObservedAssignmentRevision,
    import_revision: CurriculumImportRevision,
    source: AssignmentDefinitionSourceView,
    preview_witness: CourseScheduleWitness,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl AssignmentFastForwardCommand {
    /// Binds apply to every observation from an eligible fast-forward preview.
    pub fn from_preview(
        preview: &AssignmentFastForwardPreviewView,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        if !matches!(preview.decision, AssignmentFastForwardDecision::Eligible) {
            return Err(CurriculumAdoptionCommandError::FastForwardNotEligible);
        }
        Ok(Self {
            course: preview.course,
            assignment: preview.assignment,
            import_revision: preview.import_revision,
            source: preview.source,
            preview_witness: preview.witness.clone(),
            idempotency_key,
        })
    }

    /// Returns the previewed teaching-course destination.
    pub fn course(&self) -> CourseReference {
        self.course
    }

    /// Returns the previewed assignment revision observation.
    pub fn assignment(&self) -> ObservedAssignmentRevision {
        self.assignment
    }

    /// Returns the previewed import baseline revision.
    pub fn import_revision(&self) -> CurriculumImportRevision {
        self.import_revision
    }

    /// Returns the previewed source revision observation.
    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }

    /// Returns the course and assignment revision witness from preview.
    pub fn preview_witness(&self) -> &CourseScheduleWitness {
        &self.preview_witness
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// A preview still requires correction or is not eligible for its requested apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurriculumAdoptionCommandError {
    /// The Instructor must resolve the preview's schedule or exact-pin correction first.
    CorrectionsRequired,
    /// Fast-forward recovery outcomes preserve the current assignment instead of applying.
    FastForwardNotEligible,
    /// A course with issued learner work has no whole-course term-shift apply action.
    TermShiftNotEligible,
}

impl std::fmt::Display for CurriculumAdoptionCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorrectionsRequired => {
                formatter.write_str("curriculum adoption preview requires correction")
            }
            Self::FastForwardNotEligible => {
                formatter.write_str("fast-forward apply requires an eligible preview")
            }
            Self::TermShiftNotEligible => {
                formatter.write_str("term shift apply requires an eligible preview")
            }
        }
    }
}

impl std::error::Error for CurriculumAdoptionCommandError {}

/// Server-owned command that preserves a divergent assignment and creates a separate draft.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateSourceDerivedAssignmentCommand {
    course: CourseReference,
    source: AssignmentDefinitionSourceView,
    preview_witness: CourseScheduleWitness,
    replacements: CurriculumPinReplacements,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl CreateSourceDerivedAssignmentCommand {
    /// Binds apply to the exact destination, source, and witness returned by preview.
    pub fn from_preview(
        preview: &SourceDerivedAssignmentPreviewView,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        require_corrected_preview(&preview.corrections, preview.pin_correction.as_ref())?;
        Ok(Self {
            course: preview.course,
            source: preview.source,
            preview_witness: preview.witness.clone(),
            replacements: preview.replacements.clone(),
            idempotency_key,
        })
    }

    /// Returns the previewed teaching-course destination.
    pub fn course(&self) -> CourseReference {
        self.course
    }

    /// Returns the previewed source revision observation.
    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }

    /// Returns the course and assignment revision witness from preview.
    pub fn preview_witness(&self) -> &CourseScheduleWitness {
        &self.preview_witness
    }

    /// Returns the exact previewed public-question substitutions.
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    /// Returns the opaque completed-retry binding.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

fn require_corrected_preview<T>(
    schedule_corrections: &[T],
    pin_correction: Option<&super::UnavailablePinRecoveryAction>,
) -> Result<(), CurriculumAdoptionCommandError> {
    if schedule_corrections.is_empty() && pin_correction.is_none() {
        Ok(())
    } else {
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    }
}
