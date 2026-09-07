// Browser contract for the M11 Student Assignment Access and initial delivery.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { StudentResponse } from "../../generated/api/StudentResponse";

export type AssignmentStartDecision =
  "may_start" | "not_yet_available" | "closed" | "attempt_limit_reached" | "late_work_refused";

/** Server-calculated access for the signed-in Student only. */
export interface LiveAssignmentAccess {
  readonly startDecision: AssignmentStartDecision;
}

/** Initial or resumed Assignment Attempt presentation. M12 adds response controls. */
export interface LiveAssignmentAttempt {
  readonly assignment: AssignmentReference;
  readonly attemptNumber: number;
  readonly resumed: boolean;
  readonly title: string;
  readonly instructions: string;
  readonly questions: ReadonlyArray<QuestionPresentation>;
}

/**
 * The deliberately small M13 acknowledgement for one already-issued native
 * PLE presentation. It does not disclose the private Question Attempt,
 * Question Submission, response, result, or grading receipt.
 */
export interface LiveNativePleSubmissionAcknowledgement {
  readonly presentationNonce: string;
  readonly gradingState: "pending";
}

/** Same-origin M11 Student-only access and start boundary. */
export interface LiveAssignmentAttemptIssuanceClient {
  readonly getLiveAssignmentAccess: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<LiveAssignmentAccess>;
  readonly startLiveAssignment: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<LiveAssignmentAttempt>;
  /**
   * Submits one format-valid response through the public issued-presentation
   * capability. Course, Assignment, and nonce are all re-authorized server-side.
   */
  readonly submitLiveNativePleResponse: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    presentationNonce: string,
    response: StudentResponse,
  ) => Promise<LiveNativePleSubmissionAcknowledgement>;
}
