// Browser contract for the current Student Course Landing projection.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { AssignmentAttemptCompletion } from "../../generated/api/AssignmentAttemptCompletion";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { LiveAssignmentAttemptScore } from "./assignment_attempt_issuance";

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

/** One current Student-visible Assignment with self-only, answer-free progress. */
export interface LiveStudentAssignmentLandingSummary {
  readonly reference: AssignmentReference;
  readonly title: string;
  readonly assignmentAttemptNumber: number | null;
  readonly assignmentAttemptCompletion: AssignmentAttemptCompletion | null;
  readonly gradedQuestionCount: number;
  readonly questionCount: number;
  /** Omitted unless the pinned Assignment disclosure releases the current score. */
  readonly score?: LiveAssignmentAttemptScore;
}

/** Same-origin current-Student Course and Assignment landing capability. */
export interface LiveStudentCourseLandingClient {
  readonly listPendingLiveStudentCourseInvitations: () => Promise<
    ReadonlyArray<LiveStudentCourseInvitationSummary>
  >;
  readonly listLiveStudentCourses: () => Promise<ReadonlyArray<LiveStudentCourseLandingSummary>>;
  readonly listLiveStudentAssignments: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<LiveStudentAssignmentLandingSummary>>;
}
