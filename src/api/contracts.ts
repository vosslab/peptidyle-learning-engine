// contracts.ts - browser-safe DTOs at the transport boundary (MOD-CLIENT).

import type { AssignmentEnrollment } from "../../generated/api/AssignmentEnrollment";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { DisclosedFeedback } from "../../generated/api/DisclosedFeedback";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { RunId } from "../../generated/api/RunId";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceDraftSummary } from "../../generated/api/WorkspaceDraftSummary";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
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
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { Seed } from "../../generated/api/Seed";
import type { VersionId } from "../../generated/api/VersionId";
import type { AssignmentRunTiming } from "../../generated/api/AssignmentRunTiming";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";

export type { AssignmentSummary, CourseSummary };
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
  /** Course-owned whole-practice-run timer; null explicitly means Untimed. */
  readonly assignmentTiming: AssignmentRunTiming;
  /** Strong server-issued ETag; send it byte-for-byte when updating. */
  readonly revision: string;
}

/** Authorized resolution of one compact reference to a browser API identity. */
export type NavigationResolution =
  | { readonly kind: "course"; readonly courseId: CourseId }
  | {
      readonly kind: "assignment";
      readonly courseId: CourseId;
      readonly assignmentId: AssignmentId;
    }
  | { readonly kind: "run"; readonly runId: RunId }
  | { readonly kind: "workspace"; readonly workspaceId: WorkspaceId };

/** The exact mutable body accepted for assignment creation and replacement. */
export interface AssignmentCreateInput {
  readonly title: string;
  readonly questionIds: ReadonlyArray<string>;
  readonly policies: RunPolicies;
  /** Modern editor saves always state the whole-run timing choice explicitly. */
  readonly assignmentTiming: AssignmentRunTiming;
}

/** The ordinary editor save keeps each assigned Question ID unchanged. */
export interface AssignmentEditorInput {
  readonly title: string;
  readonly items: ReadonlyArray<{
    readonly id: string;
    readonly questionId: string;
    readonly position: number;
    readonly pointsPossible: string;
    readonly deliveryState: "active" | "retired";
    readonly scoringMode: "normal" | "fullCredit" | "extraCredit" | "excluded";
  }>;
  readonly policies: RunPolicies;
  readonly assignmentTiming: AssignmentRunTiming;
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
  readonly summary: StudentAssignmentSummary;
}

/** Explicit acknowledgement of an idempotent response submission. */
export interface SubmissionReceipt {
  readonly accepted: true;
  readonly attempt: QuestionAttempt;
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
  readonly envelope: QuestionEnvelope;
}

/** Server-redacted one-question outcome in a bounded run summary. */
export interface RunSummaryOutcome {
  readonly attempt: QuestionAttemptId;
  readonly assignmentPosition: number;
  readonly submittedAt: number | null;
  readonly response: StudentResponse | null;
  readonly feedback: DisclosedFeedback | null;
}

/** Current server projection; it never includes a question key, result, or release policy. */
export interface RunSummaryResponse {
  readonly course: CourseRouteData;
  readonly run: AssignmentRun;
  readonly summary: StudentAssignmentSummary;
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
  readonly assignment: AssignmentSummary;
  readonly run: AssignmentRun;
  readonly attempt: QuestionAttempt;
  /** Server-regenerated, key-free variant bound to this issued attempt. */
  readonly issuedQuestion: QuestionEnvelope;
}

/** Parameters needed to issue a fresh run. */
export interface StartRunRequest {
  readonly assignmentId: AssignmentId;
}

/** Run identity alias used where a return value is clearer than a full DTO. */
export type StartedRunId = RunId;
