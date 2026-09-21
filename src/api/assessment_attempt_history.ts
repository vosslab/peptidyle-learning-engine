// Strict browser contract for one Student-owned completed Assessment Attempt.

import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentFeedback } from "../../generated/api/StudentFeedback";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import { parseAssessmentAttemptId } from "../navigation/public_route";

export interface StudentAssessmentAttemptHistoryQuestion extends StudentFeedback {
  readonly position: number;
  /** Exact immutable Question Revision identity required for disclosed asset delivery. */
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly responseState: "submitted" | "closed";
  /** Readable recorded response, independently released from grading. */
  readonly response?: ReadonlyArray<QuestionContentBlock>;
  /** Permission to fetch the independently authorized correct-answer document. */
  readonly backendAnswerReview?: "available";
}

/** Derives the sole review route from existing public Attempt and position identities. */
export function backendAnswerReviewDocumentUrl(
  assessmentAttemptId: AssessmentAttemptId,
  position: number,
): string {
  const parsedAssessmentAttemptId = parseAssessmentAttemptId(assessmentAttemptId);
  if (parsedAssessmentAttemptId === null || !Number.isSafeInteger(position) || position < 1) {
    throw new Error("Invalid completed Assessment Attempt position");
  }
  return `/api/assessment-attempts/${parsedAssessmentAttemptId}/questions/${position}/answer-review-document`;
}

/** A no-store selected-history projection with protected fields omitted. */
export interface StudentAssessmentAttemptHistory {
  readonly assessmentAttemptId: AssessmentAttemptId;
  readonly attemptNumber: number;
  readonly course: {
    readonly id: CourseInstanceId;
    readonly shortName: string;
    readonly longName: string;
    readonly theme: CourseTheme;
  };
  readonly assessment: {
    readonly id: AssessmentId;
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
    assessmentAttemptId: AssessmentAttemptId,
  ) => Promise<StudentAssessmentAttemptHistory>;
}
