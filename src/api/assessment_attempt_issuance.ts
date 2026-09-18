// Browser contract for Student Assessment Access and initial delivery.

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";

export interface LiveAssessmentAttemptScore {
  readonly pointsEarned: number;
  readonly pointsPossible: number;
}

export interface LiveAssessmentPreviousAttempt {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly attemptNumber: number;
  readonly state: "submitted" | "closed";
  /** Omitted while grading is incomplete or disclosure withholds the score. */
  readonly score?: LiveAssessmentAttemptScore;
}

/** Server-calculated access for the signed-in Student only. */
export interface LiveAssessmentAccess {
  readonly decision: StudentAssessmentDecisionSummary;
  /** Authorized unfinished Assessment Attempt, if the Student can resume one. */
  readonly activeAssessmentAttempt: AssessmentAttemptId | null;
  readonly title: string;
  readonly assessmentType: AssessmentType;
  readonly questionCount: number;
  readonly pointsPossible: number;
  /** Complete, newest-first, answer-free owned Attempt history. */
  readonly previousAttempts: ReadonlyArray<LiveAssessmentPreviousAttempt>;
}

/** Initial or resumed Assessment Attempt presentation with its response controls. */
export interface LiveAssessmentAttempt {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly assessment: AssessmentId;
  readonly attemptNumber: number;
  readonly resumed: boolean;
  readonly title: string;
  readonly instructions: string;
  readonly questions: ReadonlyArray<QuestionPresentation>;
}

/** Same-origin Student-only access and start boundary. */
export interface LiveAssessmentAttemptIssuanceClient {
  readonly getLiveAssessmentAccess: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<LiveAssessmentAccess>;
  readonly startLiveAssessment: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<LiveAssessmentAttempt>;
}
