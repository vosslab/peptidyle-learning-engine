// Browser-safe Instructor Gradebook evidence boundary.

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { AssessmentAttemptCompletion } from "../../generated/api/AssessmentAttemptCompletion";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";

/** One answer-free progress aggregate for an active Student. */
export interface CourseGradebookStudentWork {
  readonly rosterId: string;
  readonly rosterName: string;
  readonly assessmentId: AssessmentId;
  readonly assessmentTitle: string;
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
  readonly courseId: CourseInstanceId;
  readonly studentWork: ReadonlyArray<CourseGradebookStudentWork>;
}

export type GradebookExportFormat = "csv" | "tsv";

/** Same-origin current-Instructor Gradebook read and point-export capability. */
export interface CourseGradebookClient {
  readonly getCourseGradebook: (courseInstanceId: CourseInstanceId) => Promise<CourseGradebook>;
  readonly downloadCourseGradebook: (
    courseInstanceId: CourseInstanceId,
    format: GradebookExportFormat,
  ) => Promise<Blob>;
}
