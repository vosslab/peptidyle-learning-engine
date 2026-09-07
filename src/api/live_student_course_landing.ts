// Browser contract for the current Student Course Landing projection.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

/** One current Student-visible Course Instance, without membership or progress details. */
export interface LiveStudentCourseLandingSummary {
  readonly reference: CourseInstanceReference;
  readonly title: string;
}

/** One pending Student Course Invitation, without invitation or membership details. */
export interface LiveStudentCourseInvitationSummary {
  readonly reference: CourseInstanceReference;
  readonly title: string;
}

/** One current Student-visible Assignment, without delivery or Student-work details. */
export interface LiveStudentAssignmentLandingSummary {
  readonly reference: AssignmentReference;
  readonly title: string;
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
