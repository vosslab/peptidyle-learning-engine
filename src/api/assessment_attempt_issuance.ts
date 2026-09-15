// Browser contract for Student Assessment Access and initial delivery.

import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { AssessmentAttemptReference } from "../../generated/api/AssessmentAttemptReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";

export interface LiveAssessmentAttemptScore {
  readonly pointsEarned: number;
  readonly pointsPossible: number;
}

export interface LiveAssessmentPreviousAttempt {
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly attemptNumber: number;
  readonly state: "submitted" | "closed";
  /** Omitted while grading is incomplete or disclosure withholds the score. */
  readonly score?: LiveAssessmentAttemptScore;
}

/** Server-calculated access for the signed-in Student only. */
export interface LiveAssessmentAccess {
  readonly decision: StudentAssessmentDecisionSummary;
  /** Authorized unfinished Assessment Attempt, if the Student can resume one. */
  readonly activeAssessmentAttempt: AssessmentAttemptReference | null;
  readonly title: string;
  readonly questionCount: number;
  readonly pointsPossible: number;
  /** Complete, newest-first, answer-free owned Attempt history. */
  readonly previousAttempts: ReadonlyArray<LiveAssessmentPreviousAttempt>;
}

/** Initial or resumed Assessment Attempt presentation with its response controls. */
export interface LiveAssessmentAttempt {
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly assessment: AssessmentReference;
  readonly attemptNumber: number;
  readonly resumed: boolean;
  readonly title: string;
  readonly instructions: string;
  readonly questions: ReadonlyArray<QuestionPresentation>;
}

/** Same-origin Student-only access and start boundary. */
export interface LiveAssessmentAttemptIssuanceClient {
  readonly getLiveAssessmentAccess: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
  ) => Promise<LiveAssessmentAccess>;
  readonly startLiveAssessment: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
  ) => Promise<LiveAssessmentAttempt>;
}
