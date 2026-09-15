// Browser-safe Instructor Gradebook evidence boundary.

import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { AssessmentAttemptCompletion } from "../../generated/api/AssessmentAttemptCompletion";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

/** One answer-free progress aggregate for an active Student. */
export interface CourseGradebookStudentWork {
  readonly rosterId: string;
  readonly assessmentReference: AssessmentReference;
  readonly assessmentAttemptCompletion: AssessmentAttemptCompletion | null;
  /** Derived from server time; no grading operation or queue state. */
  readonly expiredSubmitting: boolean;
  /**
   * Null while an expired Attempt is awaiting background submission. Gradebook
   * contributions can earn points with zero possible or exceed possible points.
   */
  readonly score: {
    readonly pointsEarned: number;
    readonly pointsPossible: number;
  } | null;
}

/** The complete browser projection for one current Instructor Course. */
export interface CourseGradebook {
  readonly courseReference: CourseInstanceReference;
  readonly studentWork: ReadonlyArray<CourseGradebookStudentWork>;
}

/** Same-origin current-Instructor Gradebook read capability. */
export interface CourseGradebookClient {
  readonly getCourseGradebook: (course: CourseInstanceReference) => Promise<CourseGradebook>;
}
