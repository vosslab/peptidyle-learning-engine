// Browser-safe Instructor Gradebook evidence boundary.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { AssignmentAttemptCompletion } from "../../generated/api/AssignmentAttemptCompletion";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

/** One answer-free progress aggregate for an active Student. */
export interface LiveDemoStudentWork {
  readonly rosterId: string;
  readonly assignmentReference: AssignmentReference;
  readonly assignmentAttemptCompletion: AssignmentAttemptCompletion | null;
  readonly gradedQuestionCount: number;
  readonly questionCount: number;
  readonly pointsEarned: number;
  readonly pointsPossible: number;
}

/** The complete browser projection for one current Instructor Course. */
export interface LiveDemoGradebook {
  readonly courseReference: CourseInstanceReference;
  readonly studentWork: ReadonlyArray<LiveDemoStudentWork>;
}

/** Same-origin current-Instructor Gradebook read capability. */
export interface LiveDemoGradebookClient {
  readonly getLiveDemoGradebook: (course: CourseInstanceReference) => Promise<LiveDemoGradebook>;
}
