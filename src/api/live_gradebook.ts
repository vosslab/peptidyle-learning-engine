// Browser-safe Instructor Gradebook evidence boundary.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

/** One answer-free aggregate of immutable graded Student Work. */
export interface LiveDemoGradedStudentWork {
  readonly rosterId: string;
  readonly assignmentReference: AssignmentReference;
  readonly gradedQuestionCount: number;
  readonly pointsEarned: number;
  readonly pointsPossible: number;
}

/** The complete browser projection for one current Instructor Course. */
export interface LiveDemoGradebook {
  readonly courseReference: CourseInstanceReference;
  readonly gradedStudentWork: ReadonlyArray<LiveDemoGradedStudentWork>;
}

/** Same-origin current-Instructor Gradebook read capability. */
export interface LiveDemoGradebookClient {
  readonly getLiveDemoGradebook: (course: CourseInstanceReference) => Promise<LiveDemoGradebook>;
}
