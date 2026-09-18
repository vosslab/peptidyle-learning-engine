// Browser contract for the current Student Course Landing projection.

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { AssessmentAttemptCompletion } from "../../generated/api/AssessmentAttemptCompletion";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";

/** One current Student-visible Course Instance, without membership or progress details. */
export interface LiveStudentCourseLandingSummary {
  readonly reference: CourseInstanceId;
  readonly shortName: string;
  readonly longName: string;
}

/** One pending Student Course Invitation, without invitation or membership details. */
export interface LiveStudentCourseInvitationSummary {
  readonly reference: CourseInstanceId;
  readonly shortName: string;
  readonly longName: string;
  readonly instructorDisplayName: string;
  readonly term: CourseTerm;
}

/** One current Student-visible Assessment with self-only, answer-free progress. */
export interface LiveStudentAssessmentLandingSummary {
  readonly reference: AssessmentId;
  readonly title: string;
  readonly assessmentType: AssessmentType;
  readonly decision: StudentAssessmentDecisionSummary;
  readonly assessmentAttemptNumber: number | null;
  readonly assessmentAttemptCompletion: AssessmentAttemptCompletion | null;
  readonly canResumeAssessmentAttempt: boolean;
  readonly gradedQuestionCount: number;
  /** Complete durable saved responses in the latest Attempt, without grading meaning. */
  readonly savedQuestionCount: number;
  readonly questionCount: number;
  /** Omitted unless the selected highest submitted Attempt releases its Assessment score. */
  readonly assessmentScore?: AssessmentGradeContribution;
}

/** Point contribution for one Assessment; Bonus work may contribute n / 0. */
export interface AssessmentGradeContribution {
  readonly pointsEarned: number;
  readonly pointsPossible: number;
}

/** Same-origin current-Student Course and Assessment landing capability. */
export interface LiveStudentCourseLandingClient {
  readonly listPendingLiveStudentCourseInvitations: () => Promise<
    ReadonlyArray<LiveStudentCourseInvitationSummary>
  >;
  readonly listLiveStudentCourses: () => Promise<ReadonlyArray<LiveStudentCourseLandingSummary>>;
  readonly listLiveStudentAssessments: (
    course: CourseInstanceId,
  ) => Promise<ReadonlyArray<LiveStudentAssessmentLandingSummary>>;
}
