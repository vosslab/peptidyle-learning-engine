// Strict browser contract for one Student-owned completed Assignment Attempt.

import type { AssignmentAttemptReference } from "../../generated/api/AssignmentAttemptReference";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentFeedback } from "../../generated/api/StudentFeedback";

export interface StudentAssignmentAttemptHistoryQuestion extends StudentFeedback {
  readonly position: number;
  readonly responseState: "submitted" | "closed";
  /** Readable recorded response, independently released from grading. */
  readonly response?: ReadonlyArray<QuestionContentBlock>;
}

/** A no-store selected-history projection with protected fields omitted. */
export interface StudentAssignmentAttemptHistory {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly attemptNumber: number;
  readonly course: {
    readonly reference: CourseInstanceReference;
    readonly shortName: string;
    readonly longName: string;
    readonly theme: CourseTheme;
  };
  readonly assignment: {
    readonly reference: AssignmentReference;
    readonly title: string;
  };
  readonly state: "submitted" | "closed";
  readonly score?: {
    readonly pointsEarned: number;
    readonly pointsPossible: number;
  };
  readonly questions: ReadonlyArray<StudentAssignmentAttemptHistoryQuestion>;
}

export interface StudentAssignmentAttemptHistoryClient {
  readonly getStudentAssignmentAttemptHistory: (
    assignmentAttempt: AssignmentAttemptReference,
  ) => Promise<StudentAssignmentAttemptHistory>;
}
