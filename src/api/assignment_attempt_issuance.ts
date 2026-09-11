// Browser contract for Student Assignment Access and initial delivery.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { AssignmentAttemptReference } from "../../generated/api/AssignmentAttemptReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";

export type AssignmentStartDecision =
  "may_start" | "not_yet_available" | "closed" | "attempt_limit_reached" | "late_work_refused";

/** Server-calculated access for the signed-in Student only. */
export interface LiveAssignmentAccess {
  readonly startDecision: AssignmentStartDecision;
  /** Authorized unfinished Assignment Attempt, if the Student can resume one. */
  readonly activeAssignmentAttempt: AssignmentAttemptReference | null;
}

/** Initial or resumed Assignment Attempt presentation with its response controls. */
export interface LiveAssignmentAttempt {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly assignment: AssignmentReference;
  readonly attemptNumber: number;
  readonly resumed: boolean;
  readonly title: string;
  readonly instructions: string;
  readonly questions: ReadonlyArray<QuestionPresentation>;
}

/** Same-origin Student-only access and start boundary. */
export interface LiveAssignmentAttemptIssuanceClient {
  readonly getLiveAssignmentAccess: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<LiveAssignmentAccess>;
  readonly startLiveAssignment: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<LiveAssignmentAttempt>;
}
