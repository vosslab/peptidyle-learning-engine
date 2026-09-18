// Browser contract for Course Roster Import and current roster access.

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";

/** One reviewed course-scoped roster row. */
export interface CourseRosterImportEntry {
  readonly email: string;
  readonly rosterId: string;
  readonly rosterName: string;
}

/** A bounded Course Roster Import commit. */
export interface CourseRosterImportInput {
  readonly entries: ReadonlyArray<CourseRosterImportEntry>;
}

/** The direct Teaching Team's course-scoped roster projection. */
export interface CourseRosterEntry {
  readonly rosterId: string;
  readonly rosterName: string;
  readonly state: "invitationPending" | "activeStudent";
}

/** Same-origin roster transport boundary. */
export interface LiveCourseRosterClient {
  readonly getLiveCourseRoster: (
    course: CourseInstanceId,
  ) => Promise<ReadonlyArray<CourseRosterEntry>>;
  readonly importLiveCourseRoster: (
    course: CourseInstanceId,
    input: CourseRosterImportInput,
  ) => Promise<ReadonlyArray<CourseRosterEntry>>;
  readonly claimLiveCourseInvitation: (
    course: CourseInstanceId,
  ) => Promise<{ readonly activeStudentMembership: boolean }>;
  readonly revokeLiveCourseRosterEntry: (
    course: CourseInstanceId,
    rosterId: string,
  ) => Promise<void>;
}
