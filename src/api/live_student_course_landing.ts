// Browser contract for the current Student Course Landing projection.

import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { AssessmentAttemptCompletion } from "../../generated/api/AssessmentAttemptCompletion";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { LiveAssessmentAttemptScore } from "./assessment_attempt_issuance";
import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";

/** One current Student-visible Course Instance, without membership or progress details. */
export interface LiveStudentCourseLandingSummary {
  readonly reference: CourseInstanceReference;
  readonly shortName: string;
  readonly longName: string;
}

/** One pending Student Course Invitation, without invitation or membership details. */
export interface LiveStudentCourseInvitationSummary {
  readonly reference: CourseInstanceReference;
  readonly shortName: string;
  readonly longName: string;
}

/** One current Student-visible Assessment with self-only, answer-free progress. */
export interface LiveStudentAssessmentLandingSummary {
  readonly reference: AssessmentReference;
  readonly title: string;
  readonly decision: StudentAssessmentDecisionSummary;
  readonly assessmentAttemptNumber: number | null;
  readonly assessmentAttemptCompletion: AssessmentAttemptCompletion | null;
  readonly gradedQuestionCount: number;
  readonly questionCount: number;
  /** Omitted unless the pinned Assessment disclosure releases the current score. */
  readonly score?: LiveAssessmentAttemptScore;
}

/** Same-origin current-Student Course and Assessment landing capability. */
export interface LiveStudentCourseLandingClient {
  readonly listPendingLiveStudentCourseInvitations: () => Promise<
    ReadonlyArray<LiveStudentCourseInvitationSummary>
  >;
  readonly listLiveStudentCourses: () => Promise<ReadonlyArray<LiveStudentCourseLandingSummary>>;
  readonly listLiveStudentAssessments: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<LiveStudentAssessmentLandingSummary>>;
}
