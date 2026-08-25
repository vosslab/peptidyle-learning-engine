// contracts.ts - browser-safe DTOs at the transport boundary (MOD-CLIENT).

import type { AssignmentEnrollment } from "../../generated/api/AssignmentEnrollment";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { LearnerAssignmentSummary } from "../../generated/api/LearnerAssignmentSummary";
import type { LearnerAssignmentDetail } from "../../generated/api/LearnerAssignmentDetail";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { DisclosedFeedback } from "../../generated/api/DisclosedFeedback";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { ScoringStatus } from "../../generated/api/ScoringStatus";
import type { RunCompletionStatus } from "../../generated/api/RunCompletionStatus";
import type { RunId } from "../../generated/api/RunId";
import type { LearnerAssignmentProgress } from "../../generated/api/LearnerAssignmentProgress";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceDraftSummary } from "../../generated/api/WorkspaceDraftSummary";
import type { Capability } from "../../generated/api/Capability";
import type { PublicationScope } from "../../generated/api/PublicationScope";
import type { License } from "../../generated/api/License";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { AttemptPolicy } from "../../generated/api/AttemptPolicy";
import type { TimingPolicy } from "../../generated/api/TimingPolicy";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { TenantId } from "../../generated/api/TenantId";
import type { UserId } from "../../generated/api/UserId";
import type { UserRole } from "../../generated/api/UserRole";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { Seed } from "../../generated/api/Seed";
import type { VersionId } from "../../generated/api/VersionId";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { InstructorAssignmentCurrentState } from "../../generated/api/InstructorAssignmentCurrentState";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { LearnerDisclosurePolicy } from "../../generated/api/LearnerDisclosurePolicy";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";

export type { AssignmentSummary, CourseSummary, LearnerAssignmentDetail, LearnerAssignmentSummary };
export type { GradebookSummaryRow };

/** One authorized course identity and its browser-safe appearance projection. */
export interface CourseRouteData {
  readonly summary: CourseSummary;
  readonly appearance: CourseAppearance;
}

/**
 * The instructor-only editable projection of a tenant-owned assignment.
 *
 * This intentionally carries immutable published references rather than
 * question definitions: authoring an assignment never transfers question
 * source, capability declarations, keys, grading, or learner-feedback policy
 * into the editor transport.
 */
export interface AssignmentEditorDetail extends AssignmentSummary {
  /** Course-local instructor projection; the server owns time-zone resolution. */
  readonly teachingSettings: InstructorAssignmentTeachingSettingsLocal;
  /** Server-derived current state at the response's authoritative instant. */
  readonly currentState: InstructorAssignmentCurrentState;
  /** Strong server-issued ETag; send it byte-for-byte when updating. */
  readonly revision: string;
}

/** Authorized resolution of one compact reference to a browser API identity. */
export type { NavigationResolution };

/** One safe catalog display fact returned from a server-owned item-pool sample. */
export interface PoolDrawPreviewQuestion {
  readonly questionId: string;
  readonly title: string;
}

/** Strict browser request for one saved selection group by its shared definition position. */
export interface PoolDrawPreviewRequest {
  readonly groupPosition: number;
}

/** A no-store Instructor sample of one saved pool; it is never learner activity or evidence. */
export interface PoolDrawPreview {
  readonly assignment: AssignmentReference;
  readonly revision: string;
  readonly groupPosition: number;
  readonly groupLabel: string;
  readonly drawCount: number;
  readonly ordering: "candidateOrder" | "randomized";
  readonly algorithm: "v1";
  readonly candidates: ReadonlyArray<PoolDrawPreviewQuestion>;
  readonly sampled: ReadonlyArray<PoolDrawPreviewQuestion>;
}

/**
 * One public, ordered assignment-definition entry. The browser sends compact Question IDs only;
 * the server resolves immutable publications and owns all internal identities and algorithm state.
 */
export type AssignmentEditorEntryInput =
  | {
      readonly kind: "fixed";
      readonly questionId: string;
      readonly position: number;
      readonly pointsPossible: string;
      readonly deliveryState: "active" | "retired";
      readonly scoringMode: "normal" | "fullCredit" | "extraCredit" | "excluded";
    }
  | {
      readonly kind: "selectionGroup";
      readonly candidateQuestionIds: ReadonlyArray<string>;
      readonly position: number;
      readonly drawCount: number;
      readonly pointsPerItem: string;
      readonly ordering: "candidateOrder" | "randomized";
    };

/** The exact mutable body accepted for assignment creation and complete definition replacement. */
export interface AssignmentCreateInput {
  readonly title: string;
  readonly entries: ReadonlyArray<AssignmentEditorEntryInput>;
  readonly policies: RunPolicies;
  /** Assignment-owned timing for each learner-facing disclosure field. */
  readonly disclosurePolicy: LearnerDisclosurePolicy;
}

/** A revision-checked complete replacement in the shared entry-position namespace. */
export interface AssignmentEditorInput {
  readonly title: string;
  readonly entries: ReadonlyArray<AssignmentEditorEntryInput>;
  readonly policies: RunPolicies;
  /** Assignment-owned timing for each learner-facing disclosure field. */
  readonly disclosurePolicy: LearnerDisclosurePolicy;
}

export interface AddAssignmentItemInput {
  readonly questionId: string;
  readonly position: number;
}

export interface ReplaceAssignmentItemQuestionInput {
  readonly questionId: string;
}

/** The deliberately small public request accepted when an instructor creates a course. */
export interface CourseCreateInput {
  readonly title: string;
  readonly term: CourseTerm;
}

/** One server-derived capability conflict for a selected immutable version. */
export interface AssignmentCapabilityViolation {
  readonly title: string;
  readonly questionId: string;
  readonly capability: Capability;
}

/** Cursor-paged API result. Offset pagination is intentionally absent. */
export interface CursorPage<T> {
  readonly items: ReadonlyArray<T>;
  readonly nextCursor: string | null;
}

/** Signed-in identity projection. Session credentials remain in an HttpOnly cookie. */
export interface AuthSession {
  readonly authenticated: true;
  readonly tenant: TenantId;
  readonly user: {
    readonly id: UserId;
    readonly displayName: string;
    readonly roles: ReadonlyArray<UserRole>;
  };
}

/** Confirmation that both browser authentication scopes were revoked. */
export interface SignedOutResponse {
  readonly authenticated: false;
}

/** Enrollment and its transactionally maintained student summary. */
export interface EnrollmentView {
  readonly enrollment: AssignmentEnrollment;
  /** Key-free current learner projection; score totals are omitted while withheld. */
  readonly summary: LearnerAssignmentProgress;
}

/** Learner attempt projection with the current server-owned score freshness gate. */
export interface LearnerQuestionAttempt extends QuestionAttempt {
  readonly scoringStatus: ScoringStatus;
  /** Null for a fixed item; a safe ordinal explanation for one server-selected pool item. */
  readonly poolSelection: PoolSelection | null;
}

export interface PoolSelection {
  readonly itemNumber: number;
  readonly itemCount: number;
}

/** Explicit acknowledgement of an idempotent response submission. */
export interface SubmissionReceipt {
  readonly accepted: true;
  readonly attempt: QuestionAttempt;
  readonly scoringStatus: ScoringStatus;
  /** Persisted completion state; successor absence alone is not completion evidence. */
  readonly runCompletionStatus: RunCompletionStatus;
  /** Server-redacted teaching material, or an explicit policy withholding it. */
  readonly feedback: DisclosedFeedback | null;
  readonly nextIssued: NextIssuedAttempt | null;
  /** The grade receipt is durable, but a successor has not been issued yet. */
  readonly nextPending: boolean;
}

/** Safe binding for a newly active next attempt; no provenance or source leaks. */
export interface NextIssuedAttempt {
  readonly id: QuestionAttemptId;
  readonly run: RunId;
  readonly questionVersion: VersionId;
  readonly seed: Seed;
  readonly deadline: number | null;
  readonly assignmentPosition: number;
  readonly renderedQuestionSha256: string;
}

/** Key-free envelope cached only behind its owned predecessor attempt. */
export interface PrefetchedNextQuestion {
  readonly predecessor: QuestionAttemptId;
  readonly run: RunId;
  readonly assignmentPosition: number;
  readonly questionVersion: VersionId;
  readonly seed: Seed;
  readonly renderedQuestionSha256: string;
  /** Same safe ordinal provenance used when this cached successor becomes current. */
  readonly poolSelection: PoolSelection | null;
  readonly envelope: QuestionEnvelope;
}

/** Server-redacted one-question outcome in a bounded run summary. */
export interface RunSummaryOutcome {
  readonly attempt: QuestionAttemptId;
  readonly assignmentPosition: number;
  readonly submittedAt: number | null;
  readonly response: StudentResponse | null;
  readonly feedback: DisclosedFeedback | null;
  readonly scoringStatus: ScoringStatus;
}

/** Current server projection; it never includes a question key, result, or release policy. */
export interface RunSummaryResponse {
  readonly course: CourseRouteData;
  readonly run: AssignmentRun;
  /** Server-derived learner progress, never a policy, clock, or enrollment identifier. */
  readonly summary: LearnerAssignmentProgress;
  readonly practiceAllowed: boolean;
  readonly outcomes: CursorPage<RunSummaryOutcome>;
}

export interface FeedbackReleaseResponse {
  readonly released: true;
}

/** Strong ETag issued by the workspace route; pass it back byte-for-byte on an update. */
export interface WorkspaceDraftDetail {
  readonly draft: DraftQuestionDefinition;
  readonly revision: string;
}

export type WorkspaceDraftPage = CursorPage<WorkspaceDraftSummary>;

export interface PublicationViolation {
  readonly workspace: string;
  readonly title: string;
  readonly capability: Capability;
}

export interface PublicationValidationReport {
  readonly violations: ReadonlyArray<PublicationViolation>;
}

/** A persisted draft cannot be prepared for publication, without a capability violation payload. */
export interface PublicationReadinessFailure {
  readonly kind: "readinessFailure";
  readonly message: string;
}

/** The validation endpoint's browser contract: capability report or honest readiness refusal. */
export type PublicationValidationResponse =
  | {
      readonly kind: "capabilityReport";
      readonly revision: string;
      readonly violations: ReadonlyArray<PublicationViolation>;
    }
  | PublicationReadinessFailure;

export interface PublicationDiff {
  readonly draftRevision: number;
  readonly revision: string;
  readonly baseline: "newQuestion";
  readonly current: PublicationSemanticProjection;
  readonly changed: ReadonlyArray<
    | "sourceBackend"
    | "title"
    | "prompt"
    | "response"
    | "attemptPolicy"
    | "timingPolicy"
    | "randomization"
    | "metadata"
  >;
}

/** Safe semantic comparison only; source locators, grading, and keys never cross this boundary. */
export interface PublicationSemanticProjection {
  readonly sourceBackend: QuestionBackend;
  readonly title: string;
  readonly prompt: { readonly blocks: ReadonlyArray<PublicationPromptBlockKind> };
  readonly response: {
    readonly kind: PublicationResponseKind;
    readonly optionCount: number | null;
  };
  readonly attemptPolicy: AttemptPolicy;
  readonly timingPolicy: TimingPolicy;
  readonly randomization: { readonly kind: "static" | "seeded" };
  readonly metadata: {
    readonly tags: ReadonlyArray<string>;
    readonly taxonomy: ReadonlyArray<TaxonomyTerm>;
    readonly license: License;
    readonly language: string;
  };
}

export type PublicationPromptBlockKind = "text" | "math" | "image" | "code" | "table";
export type PublicationResponseKind =
  | "numeric"
  | "multipleChoice"
  | "shortText"
  | "multiBlank"
  | "matching"
  | "ordering"
  | "hotspot"
  | "fileUpload"
  | "externalTool";

export interface PublicationResult {
  readonly summary: CatalogProblemSummary;
}

export interface PublicationRequest {
  readonly scope: PublicationScope;
  readonly byline: CatalogProblemSummary["byline"];
}

/**
 * Per-attempt route to a server-owned external-tool broker.
 *
 * This is deliberately a same-origin path only. It carries no provider URL,
 * token, correlation identifier, score, or immutable question content.
 */
export interface ExternalToolLaunch {
  readonly launchUrl: string;
}

/** Everything the reference run screen needs from one cached query. */
export interface RunScreenData {
  readonly course: CourseRouteData;
  /** Learner-safe assignment projection; no policy or ownership inputs. */
  readonly assignment: LearnerAssignmentSummary;
  readonly run: AssignmentRun;
  readonly attempt: LearnerQuestionAttempt;
  /** Server-regenerated, key-free variant bound to this issued attempt. */
  readonly issuedQuestion: QuestionEnvelope;
}

/** Run identity alias used where a return value is clearer than a full DTO. */
export type StartedRunId = RunId;
