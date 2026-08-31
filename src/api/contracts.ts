// contracts.ts - browser-safe DTOs at the transport boundary (MOD-CLIENT).

import type { AssignmentAttempt } from "../../generated/api/AssignmentAttempt";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { StudentAssignmentLandingSummary } from "../../generated/api/StudentAssignmentLandingSummary";
import type { StudentAssignmentDetail } from "../../generated/api/StudentAssignmentDetail";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { StudentFeedback } from "../../generated/api/StudentFeedback";
import type { StudentQuestionAttemptView } from "../../generated/api/StudentQuestionAttemptView";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { AssignmentEntryId } from "../../generated/api/AssignmentEntryId";
import type { IssuedQuestionId } from "../../generated/api/IssuedQuestionId";
import type { QuestionVersionReference } from "../../generated/api/QuestionVersionReference";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { AssignmentScoringState } from "../../generated/api/AssignmentScoringState";
import type { AssignmentStatus } from "../../generated/api/AssignmentStatus";
import type { AssignmentAttemptCompletion } from "../../generated/api/AssignmentAttemptCompletion";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceDraftSummary } from "../../generated/api/WorkspaceDraftSummary";
import type { Capability } from "../../generated/api/Capability";
import type { License } from "../../generated/api/License";
import type { QuestionClassification } from "../../generated/api/QuestionClassification";
import type { QuestionAttemptLimit } from "../../generated/api/QuestionAttemptLimit";
import type { QuestionAttemptTimeLimit } from "../../generated/api/QuestionAttemptTimeLimit";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { AccountId } from "../../generated/api/AccountId";
import type { AccountRole } from "../../generated/api/AccountRole";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { InstructorAssignmentWorkingCopyDefinitionLocal } from "../../generated/api/InstructorAssignmentWorkingCopyDefinitionLocal";
import type { InstructorAssignmentCurrentState } from "../../generated/api/InstructorAssignmentCurrentState";
import type { QuestionSummary } from "../../generated/api/QuestionSummary";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { AssignmentReleaseValidation } from "../../generated/api/AssignmentReleaseValidation";
import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { CreateAssignmentRequest } from "../../generated/api/CreateAssignmentRequest";
import type { ReplaceAssignmentPoliciesRequest } from "../../generated/api/ReplaceAssignmentPoliciesRequest";
import type { ReplaceAssignmentFixedItemRequest } from "../../generated/api/ReplaceAssignmentFixedItemRequest";

export type {
  AssignmentSummary,
  CourseSummary,
  InstructorStudentView,
  StudentAssignmentDetail,
  StudentAssignmentLandingSummary,
};
export type { CreateAssignmentRequest as AssignmentCreateInput };

/** The HTTP client adds the exact current Assignment Working Copy edit precondition. */
export type AssignmentPoliciesInput = Omit<ReplaceAssignmentPoliciesRequest, "baseEditNumber">;
/** The HTTP client adds the exact current Assignment Working Copy edit precondition. */
export type ReplaceAssignmentFixedItemInput = Omit<
  ReplaceAssignmentFixedItemRequest,
  "baseEditNumber"
>;

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
  /** Stable Assignment Status; release selection stays outside editable content. */
  readonly assignmentStatus: AssignmentStatus;
  /** Course-local instructor projection; the server owns time-zone resolution. */
  readonly assignmentWorkingCopyDefinition: InstructorAssignmentWorkingCopyDefinitionLocal;
  /** Server-derived current state at the response's authoritative instant. */
  readonly currentState: InstructorAssignmentCurrentState;
  /** Closed, server-derived release blockers for this Assignment Working Copy. */
  readonly assignmentReleaseValidation: AssignmentReleaseValidation;
  /** Strong server-issued ETag; send it byte-for-byte when updating. */
  readonly revision: string;
}

/** Authorized resolution of one compact reference to a browser API identity. */
export type { NavigationResolution };

/** One safe Question Library display fact returned from a server-owned Question Pool Preview. */
export interface QuestionPoolPreviewQuestion {
  readonly questionId: string;
  readonly title: string;
}

/** Strict browser request for one saved Question Pool by its Assignment Entry reference. */
export interface QuestionPoolPreviewRequest {
  readonly assignmentEntryId: string;
}

/** A no-store Instructor sample of one saved pool; it is never student activity or evidence. */
export interface QuestionPoolPreview {
  readonly assignment: AssignmentReference;
  readonly revision: string;
  readonly assignmentEntryId: string;
  readonly questionPoolLabel: string;
  readonly selectionCount: number;
  readonly selectionRule: {
    readonly selectedQuestionOrder: "candidateOrder" | "randomOrder";
  };
  readonly candidates: ReadonlyArray<QuestionPoolPreviewQuestion>;
  readonly selected: ReadonlyArray<QuestionPoolPreviewQuestion>;
}

/**
 * One public, ordered assignment-definition entry. The browser sends compact Question IDs only;
 * the server resolves immutable publications and owns all internal identities and selection mechanics.
 */
export type AssignmentEditorEntryInput =
  | {
      readonly kind: "fixedQuestion";
      readonly questionId: string;
      readonly pointsPossible: string;
      readonly availability: "available" | "retired";
      readonly scoringRule: "normal" | "fullCredit" | "extraCredit" | "excluded";
    }
  | {
      readonly kind: "questionPool";
      readonly candidateQuestionIds: ReadonlyArray<string>;
      readonly availability: "available" | "retired";
      readonly scoringRule: "normal" | "fullCredit" | "extraCredit" | "excluded";
      readonly selectionCount: number;
      readonly pointsPerItem: string;
      readonly selectionRule: {
        readonly selectedQuestionOrder: "candidateOrder" | "randomOrder";
      };
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
export interface StudentQuestionAttempt extends StudentQuestionAttemptView {
  readonly assignmentScoringState: AssignmentScoringState;
  /** Null for a fixed item; a safe ordinal explanation for one server-selected pool item. */
  readonly questionPoolSelectionPosition: QuestionPoolSelectionPosition | null;
}

/** Browser-safe ordinal position within one server-owned Question Pool Selection. */
export interface QuestionPoolSelectionPosition {
  readonly itemNumber: number;
  readonly itemCount: number;
}

/**
 * Browser-safe Issued Question identity and exact published Question Revision.
 *
 * The durable source selection and candidate stay in server-held Student Work
 * records. Student delivery identifies pooled work only through
 * QuestionPoolSelectionPosition.
 */
export interface StudentIssuedQuestion {
  readonly id: IssuedQuestionId;
  readonly assignmentAttempt: AssignmentAttemptId;
  readonly assignmentEntry: AssignmentEntryId;
  readonly definitionEntryIndex: number;
  readonly issuedPosition: number;
  readonly reference: QuestionVersionReference;
  readonly statisticsEligible: boolean;
}

/** Explicit acknowledgement of an idempotent response submission. */
export interface QuestionSubmissionReceipt {
  readonly accepted: true;
  readonly attemptId: QuestionAttemptId;
}

/** A Question Submission Receipt after grading has produced a browser-safe result. */
export interface GradedQuestionSubmissionReceipt extends QuestionSubmissionReceipt {
  readonly attempt: StudentQuestionAttemptView;
  readonly assignmentScoringState: AssignmentScoringState;
  /** Persisted completion state; successor absence alone is not completion evidence. */
  readonly assignmentAttemptCompletion: AssignmentAttemptCompletion;
  /** Server-redacted teaching material, or an explicit policy withholding it. */
  readonly feedback: StudentFeedback | null;
  readonly nextIssued: NextIssuedAttempt | null;
  /** The grade receipt is durable, but a successor has not been issued yet. */
  readonly nextPending: boolean;
}

/**
 * The closed Student acknowledgement returned by submission and status routes.
 * Pending alternatives deliberately omit answers, feedback, results, successors, and scores.
 */
export type QuestionSubmissionGradingState = "pending" | "graded" | "instructorAttention";

/** Accepted Question Submission plus its separate current grading state. */
export type QuestionSubmissionAcknowledgement =
  | {
      readonly receipt: GradedQuestionSubmissionReceipt;
      readonly gradingState: "graded";
    }
  | {
      readonly receipt: QuestionSubmissionReceipt;
      readonly gradingState: "pending";
      readonly nextAction: "check_status";
    }
  | {
      readonly receipt: QuestionSubmissionReceipt;
      readonly gradingState: "instructorAttention";
      readonly nextAction: "check_status";
    };

/** Safe binding for a newly active next attempt; no source record or source material leaks. */
export interface NextIssuedAttempt {
  readonly id: QuestionAttemptId;
  readonly issuedQuestion: StudentIssuedQuestion;
  readonly seed: number;
  readonly deadline: number | null;
  readonly renderedQuestionSha256: string;
}

/** Key-free envelope cached only behind its owned predecessor attempt. */
export interface PrefetchedNextQuestion {
  readonly predecessor: QuestionAttemptId;
  readonly issuedQuestion: StudentIssuedQuestion;
  readonly seed: number;
  readonly renderedQuestionSha256: string;
  /** Same safe ordinal provenance used when this cached successor becomes current. */
  readonly questionPoolSelectionPosition: QuestionPoolSelectionPosition | null;
  readonly envelope: QuestionPresentation;
}

/** Server-redacted one-question outcome in a bounded Assignment Attempt summary. */
export interface AssignmentAttemptSummaryOutcome {
  readonly attempt: QuestionAttemptId;
  readonly issuedQuestion: StudentIssuedQuestion;
  readonly submittedAt: number | null;
  readonly response: StudentResponse | null;
  readonly feedback: StudentFeedback | null;
  readonly assignmentScoringState: AssignmentScoringState;
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

/** Answer-free review of the saved Question Working Copy proposed for publication. */
export interface QuestionPublicationReview {
  readonly workingCopyEditNumber: number;
  readonly revision: string;
  readonly baseQuestion: "newQuestion";
  readonly current: QuestionPublicationReviewSummary;
  readonly changed: ReadonlyArray<
    | "sourceBackend"
    | "title"
    | "prompt"
    | "response"
    | "questionAttemptLimit"
    | "questionAttemptTimeLimit"
    | "questionVariationDefinition"
    | "metadata"
  >;
}

/** Safe review summary; source locators, grading, and keys remain server-held. */
export interface QuestionPublicationReviewSummary {
  readonly sourceBackend: QuestionBackend;
  readonly title: string;
  readonly prompt: { readonly blocks: ReadonlyArray<PublicationPromptBlockKind> };
  readonly response: {
    readonly kind: PublicationResponseKind;
    readonly optionCount: number | null;
  };
  readonly questionAttemptLimit: QuestionAttemptLimit;
  readonly questionAttemptTimeLimit: QuestionAttemptTimeLimit;
  readonly questionVariationDefinition: { readonly kind: "static" | "seeded" };
  readonly metadata: {
    readonly tags: ReadonlyArray<string>;
    readonly classifications: ReadonlyArray<QuestionClassification>;
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
  | "externalTool";

export interface PublicationResult {
  readonly summary: QuestionSummary;
}

export interface PublicationRequest {
  readonly byline: QuestionSummary["byline"];
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
  readonly issuedQuestion: QuestionPresentation;
}

/** Assignment Attempt identity alias used where a return value is clearer than a full DTO. */
export type StartedAssignmentAttemptId = AssignmentAttemptId;
