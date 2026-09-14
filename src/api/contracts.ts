// contracts.ts - browser-safe DTOs at the application transport boundary.

import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { StudentAssignmentLandingSummary } from "../../generated/api/StudentAssignmentLandingSummary";
import type { StudentAssignmentDetail } from "../../generated/api/StudentAssignmentDetail";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { StudentQuestionAttemptView } from "../../generated/api/StudentQuestionAttemptView";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { AssignmentEntryId } from "../../generated/api/AssignmentEntryId";
import type { IssuedQuestionId } from "../../generated/api/IssuedQuestionId";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import type { AssignmentScoringState } from "../../generated/api/AssignmentScoringState";
import type { AssignmentStatus } from "../../generated/api/AssignmentStatus";
import type { Capability } from "../../generated/api/Capability";
import type { AccountId } from "../../generated/api/AccountId";
import type { ProductRole } from "../../generated/api/ProductRole";
import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import type { InstructorAssignmentAuthoredContentLocal } from "../../generated/api/InstructorAssignmentAuthoredContentLocal";
import type { InstructorAssignmentAvailabilityView } from "../../generated/api/InstructorAssignmentAvailabilityView";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { AssignmentEditNumber } from "../../generated/api/AssignmentEditNumber";
import type { AssignmentReleaseValidation } from "../../generated/api/AssignmentReleaseValidation";
import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { CreateAssignmentRequest } from "../../generated/api/CreateAssignmentRequest";

export type {
  AssignmentSummary,
  CourseSummary,
  InstructorStudentView,
  StudentAssignmentDetail,
  StudentAssignmentLandingSummary,
};
export type { CreateAssignmentRequest as AssignmentCreateInput };

/** Questions-owned browser input; readonly collections retain page draft ownership. */
export interface AssignmentContentInput {
  readonly title: string;
  readonly entries: ReadonlyArray<AssignmentEditorEntryInput>;
}

/** One authorized Course Route View with its summary and browser-safe appearance. */
export interface CourseRouteView {
  readonly summary: CourseSummary;
  readonly appearance: CourseAppearanceView;
}

/**
 * The instructor-only editable Instructor Assignment Authored Content Local.
 *
 * This intentionally carries immutable published references rather than
 * complete Question Revisions: authoring an assignment never transfers question
 * source, capability declarations, keys, grading, or student-feedback policy
 * into the editor transport.
 */
export interface AssignmentEditorDetail extends AssignmentSummary {
  /** Stable Assignment Status; release selection stays outside editable content. */
  readonly assignmentStatus: AssignmentStatus;
  /** Zone-free Instructor Assignment Authored Content Local; the server owns trusted-zone resolution. */
  readonly assignmentAuthoredContent: InstructorAssignmentAuthoredContentLocal;
  /** Server-derived Assignment Availability View at the response's authoritative instant. */
  readonly assignmentAvailability: InstructorAssignmentAvailabilityView;
  /** Closed, server-derived release blockers for this Assignment. */
  readonly assignmentReleaseValidation: AssignmentReleaseValidation;
  /** Strong server-issued ETag; send it byte-for-byte when updating. */
  readonly revision: string;
}

/** Authorized resolution of one compact reference to a browser API identity. */
export type { NavigationResolution };

/** One safe Question Library display fact returned from a server-owned Question Pool Preview. */
export interface QuestionPoolPreviewItem {
  readonly questionId: string;
  readonly questionTitle: string;
}

/** Strict browser request for one saved Question Pool by its Assignment Entry reference. */
export interface QuestionPoolPreviewRequest {
  readonly assignmentEntryId: string;
}

/** A no-store Instructor sample of one saved pool; it is never student activity or evidence. */
export interface QuestionPoolPreview {
  readonly assignment: AssignmentReference;
  readonly editNumber: AssignmentEditNumber;
  readonly assignmentEntryId: string;
  readonly questionPoolLabel: string;
  readonly selectionCount: number;
  readonly selectionRule: {
    readonly selectedQuestionOrder: "questionPoolOrder" | "randomOrder";
  };
  readonly items: ReadonlyArray<QuestionPoolPreviewItem>;
  readonly selectedItems: ReadonlyArray<QuestionPoolPreviewItem>;
}

/**
 * One public, ordered Assignment Content entry. The browser sends compact Question IDs only;
 * the server resolves immutable publications and owns all internal identities and selection mechanics.
 */
export type AssignmentEditorEntryInput =
  | {
      readonly kind: "fixedQuestion";
      readonly questionId: string;
      readonly pointsPossible: string;
      readonly availability: "available" | "retired";
      readonly scoringRule: "normal" | "fullCredit" | "extraCredit" | "excluded";
      readonly questionAttemptLimit: import("../../generated/api/QuestionAttemptLimit").QuestionAttemptLimit;
      readonly questionAttemptTimeLimit: import("../../generated/api/QuestionAttemptTimeLimit").QuestionAttemptTimeLimit;
    }
  | {
      readonly kind: "questionPool";
      readonly questionIds: ReadonlyArray<string>;
      readonly availability: "available" | "retired";
      readonly scoringRule: "normal" | "fullCredit" | "extraCredit" | "excluded";
      readonly selectionCount: number;
      readonly pointsPerItem: string;
      readonly selectionRule: {
        readonly selectedQuestionOrder: "questionPoolOrder" | "randomOrder";
      };
      readonly questionAttemptLimit: import("../../generated/api/QuestionAttemptLimit").QuestionAttemptLimit;
      readonly questionAttemptTimeLimit: import("../../generated/api/QuestionAttemptTimeLimit").QuestionAttemptTimeLimit;
    };

/** One server-derived capability conflict for a selected immutable version. */
export interface AssignmentCapabilityViolation {
  readonly questionTitle: string;
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
    readonly productRole: ProductRole;
  };
}

/** Confirmation that both browser authentication scopes were revoked. */
export interface SignedOutResponse {
  readonly authenticated: false;
}

/** Student Question Attempt View with the current server-owned score freshness gate. */
export interface StudentQuestionAttempt extends StudentQuestionAttemptView {
  readonly assignmentScoringState: AssignmentScoringState;
  /** Null for a Fixed Question Assignment Entry; otherwise the safe selected-Question position. */
  readonly questionPoolSelectionPosition: QuestionPoolSelectionPosition | null;
}

/** Browser-safe ordinal position within one server-owned Question Pool Selection. */
export interface QuestionPoolSelectionPosition {
  readonly selectedQuestionNumber: number;
  readonly selectedQuestionCount: number;
}

/**
 * Browser-safe Issued Question identity and exact published Question Revision.
 *
 * The durable source selection and Question Pool Item stay in server-held Student Work
 * records. Student delivery identifies pooled work only through
 * QuestionPoolSelectionPosition.
 */
export interface StudentIssuedQuestion {
  readonly id: IssuedQuestionId;
  readonly assignmentAttempt: AssignmentAttemptId;
  readonly assignmentEntry: AssignmentEntryId;
  readonly assignmentContentEntryIndex: number;
  readonly issuedPosition: number;
  readonly reference: QuestionRevisionReference;
  readonly questionStatisticsEligibility: boolean;
}

/** Receipt for an Instructor command that releases Student Feedback for one attempt. */
export interface StudentFeedbackReleaseResponse {
  readonly released: true;
}

/**
 * Per-attempt route for the server-owned iMathAS Question Backend Transport.
 *
 * This is deliberately a same-origin path only. It carries no iMathAS URL,
 * token, correlation identifier, score, or immutable question content.
 */
export interface ImathasQuestionBackendLaunch {
  readonly launchUrl: string;
}
