// contracts.ts - browser-safe DTOs at the transport boundary (MOD-CLIENT).

import type { AssignmentAttempt } from "../../generated/api/AssignmentAttempt";
import type { IssuedQuestion } from "../../generated/api/IssuedQuestion";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { StudentAssignmentLandingSummary } from "../../generated/api/StudentAssignmentLandingSummary";
import type { StudentAssignmentDetail } from "../../generated/api/StudentAssignmentDetail";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { DisclosedFeedback } from "../../generated/api/DisclosedFeedback";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { ScoringStatus } from "../../generated/api/ScoringStatus";
import type { AssignmentAttemptCompletion } from "../../generated/api/AssignmentAttemptCompletion";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceDraftSummary } from "../../generated/api/WorkspaceDraftSummary";
import type { Capability } from "../../generated/api/Capability";
import type { License } from "../../generated/api/License";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { AttemptPolicy } from "../../generated/api/AttemptPolicy";
import type { TimingPolicy } from "../../generated/api/TimingPolicy";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { AccountId } from "../../generated/api/AccountId";
import type { AccountRole } from "../../generated/api/AccountRole";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { InstructorAssignmentCurrentState } from "../../generated/api/InstructorAssignmentCurrentState";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { AssignmentPublicationReadiness } from "../../generated/api/AssignmentPublicationReadiness";
import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { CreateAssignmentDraftRequest } from "../../generated/api/CreateAssignmentDraftRequest";
import type { ReplaceAssignmentPoliciesRequest } from "../../generated/api/ReplaceAssignmentPoliciesRequest";
import type { ReplaceAssignmentFixedItemRequest } from "../../generated/api/ReplaceAssignmentFixedItemRequest";

export type {
  AssignmentSummary,
  CourseSummary,
  InstructorStudentView,
  StudentAssignmentDetail,
  StudentAssignmentLandingSummary,
};
export type {
  CreateAssignmentDraftRequest as AssignmentDraftInput,
  ReplaceAssignmentPoliciesRequest as AssignmentPoliciesInput,
  ReplaceAssignmentFixedItemRequest as ReplaceAssignmentFixedItemInput,
};

/** Questions-owned browser input; readonly collections retain page draft ownership. */
export interface AssignmentContentInput {
  readonly title: string;
  readonly entries: ReadonlyArray<AssignmentEditorEntryInput>;
}

/** One authorized course identity and its browser-safe appearance projection. */
export interface CourseRouteData {
  readonly summary: CourseSummary;
  readonly appearance: CourseAppearance;
}

/**
 * The instructor-only editable projection of a course-owned assignment.
 *
 * This intentionally carries immutable published references rather than
 * question definitions: authoring an assignment never transfers question
 * source, capability declarations, keys, grading, or student-feedback policy
 * into the editor transport.
 */
export interface AssignmentEditorDetail extends AssignmentSummary {
  /** Course-local instructor projection; the server owns time-zone resolution. */
  readonly teachingSettings: InstructorAssignmentTeachingSettingsLocal;
  /** Server-derived current state at the response's authoritative instant. */
  readonly currentState: InstructorAssignmentCurrentState;
  /** Closed, server-derived publication blockers for the current definition. */
  readonly publicationReadiness: AssignmentPublicationReadiness;
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

/** A no-store Instructor sample of one saved pool; it is never student activity or evidence. */
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

/** Resolved browser-safe Authenticated Session. The credential remains HttpOnly. */
export interface AuthenticatedSession {
  readonly authenticated: true;
  readonly account: {
    readonly id: AccountId;
    readonly role: AccountRole;
  };
}

/** Confirmation that both browser authentication scopes were revoked. */
export interface SignedOutResponse {
  readonly authenticated: false;
}

/** Student attempt projection with the current server-owned score freshness gate. */
export interface StudentQuestionAttempt extends QuestionAttempt {
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
  readonly assignmentAttemptCompletion: AssignmentAttemptCompletion;
  /** Server-redacted teaching material, or an explicit policy withholding it. */
  readonly feedback: DisclosedFeedback | null;
  readonly nextIssued: NextIssuedAttempt | null;
  /** The grade receipt is durable, but a successor has not been issued yet. */
  readonly nextPending: boolean;
}

/**
 * The closed Student acknowledgement returned by submission and status routes.
 * Pending alternatives deliberately omit answers, feedback, results, successors, and scores.
 */
export type StudentSubmissionStatus =
  | ({ readonly kind: "completed" } & SubmissionReceipt)
  | {
      readonly kind: "accepted_pending";
      readonly accepted: true;
      readonly attemptId: QuestionAttemptId;
      readonly automatedGradingStatus: "pending";
      readonly nextAction: "check_status";
    }
  | {
      readonly kind: "instructor_attention";
      readonly accepted: true;
      readonly attemptId: QuestionAttemptId;
      readonly automatedGradingStatus: "instructor_attention";
      readonly nextAction: "check_status";
    };

/** Safe binding for a newly active next attempt; no provenance or source leaks. */
export interface NextIssuedAttempt {
  readonly id: QuestionAttemptId;
  readonly issuedQuestion: IssuedQuestion;
  readonly seed: number;
  readonly deadline: number | null;
  readonly renderedQuestionSha256: string;
}

/** Key-free envelope cached only behind its owned predecessor attempt. */
export interface PrefetchedNextQuestion {
  readonly predecessor: QuestionAttemptId;
  readonly issuedQuestion: IssuedQuestion;
  readonly seed: number;
  readonly renderedQuestionSha256: string;
  /** Same safe ordinal provenance used when this cached successor becomes current. */
  readonly poolSelection: PoolSelection | null;
  readonly envelope: QuestionEnvelope;
}

/** Server-redacted one-question outcome in a bounded Assignment Attempt summary. */
export interface AssignmentAttemptSummaryOutcome {
  readonly attempt: QuestionAttemptId;
  readonly issuedQuestion: IssuedQuestion;
  readonly submittedAt: number | null;
  readonly response: StudentResponse | null;
  readonly feedback: DisclosedFeedback | null;
  readonly scoringStatus: ScoringStatus;
}

/** Current server projection; it never includes a question key, result, or release policy. */
export interface AssignmentAttemptSummaryResponse {
  readonly course: CourseRouteData;
  readonly assignmentAttempt: AssignmentAttempt;
  /** Server-derived student progress, never a policy, clock, or Student Record identifier. */
  readonly summary: AssignmentProgress;
  readonly outcomes: CursorPage<AssignmentAttemptSummaryOutcome>;
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

/** Everything the reference Assignment Attempt screen needs from one cached query. */
export interface AssignmentAttemptScreenData {
  readonly course: CourseRouteData;
  /** Student-safe assignment projection; no policy or ownership inputs. */
  readonly assignment: StudentAssignmentLandingSummary;
  readonly assignmentAttempt: AssignmentAttempt;
  readonly attempt: StudentQuestionAttempt;
  /** Server-regenerated, key-free variant bound to this issued attempt. */
  readonly issuedQuestion: QuestionEnvelope;
}

/** Assignment Attempt identity alias used where a return value is clearer than a full DTO. */
export type StartedAssignmentAttemptId = AssignmentAttemptId;
