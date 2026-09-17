// Strict browser contract for one Student-owned completed Assessment Attempt.

import type { AssessmentAttemptReference } from "../../generated/api/AssessmentAttemptReference";
import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentFeedback } from "../../generated/api/StudentFeedback";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import { parseAssessmentAttemptReference } from "../navigation/public_route";

export interface StudentAssessmentAttemptHistoryQuestion extends StudentFeedback {
  readonly position: number;
  /** Exact immutable Question Revision identity required for disclosed asset delivery. */
  readonly questionRevision: QuestionRevisionReference;
  readonly responseState: "submitted" | "closed";
  /** Readable recorded response, independently released from grading. */
  readonly response?: ReadonlyArray<QuestionContentBlock>;
  /** Permission to fetch the independently authorized correct-answer document. */
  readonly backendAnswerReview?: "available";
}

/** Derives the sole review route from existing public Attempt and position identities. */
export function backendAnswerReviewDocumentUrl(
  assessmentAttempt: AssessmentAttemptReference,
  position: number,
): string {
  const reference = parseAssessmentAttemptReference(assessmentAttempt);
  if (reference === null || !Number.isSafeInteger(position) || position < 1) {
    throw new Error("Invalid completed Assessment Attempt position");
  }
  return `/api/assessment-attempts/${reference}/questions/${position}/answer-review-document`;
}

/** A no-store selected-history projection with protected fields omitted. */
export interface StudentAssessmentAttemptHistory {
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly attemptNumber: number;
  readonly course: {
    readonly reference: CourseInstanceReference;
    readonly shortName: string;
    readonly longName: string;
    readonly theme: CourseTheme;
  };
  readonly assessment: {
    readonly reference: AssessmentReference;
    readonly title: string;
  };
  readonly state: "submitted" | "closed";
  readonly score?: {
    readonly pointsEarned: number;
    readonly pointsPossible: number;
  };
  readonly questions: ReadonlyArray<StudentAssessmentAttemptHistoryQuestion>;
}

export interface StudentAssessmentAttemptHistoryClient {
  readonly getStudentAssessmentAttemptHistory: (
    assessmentAttempt: AssessmentAttemptReference,
  ) => Promise<StudentAssessmentAttemptHistory>;
}
