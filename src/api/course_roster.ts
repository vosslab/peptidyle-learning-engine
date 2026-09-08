// Browser contract for Course Roster Import and current roster access.

import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

/** One reviewed course-scoped roster row. */
export interface CourseRosterImportEntry {
  readonly email: string;
  readonly rosterId: string;
}

/** A bounded Course Roster Import commit. */
export interface CourseRosterImportInput {
  readonly entries: ReadonlyArray<CourseRosterImportEntry>;
}

/** The direct Teaching Team's course-scoped roster projection. */
export interface CourseRosterEntry {
  readonly rosterId: string;
  readonly rosterEmail: string;
  readonly state: "invitationPending" | "activeStudent";
}

/** Same-origin roster transport boundary. */
export interface LiveCourseRosterClient {
  readonly getLiveCourseRoster: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<CourseRosterEntry>>;
  readonly importLiveCourseRoster: (
    course: CourseInstanceReference,
    input: CourseRosterImportInput,
  ) => Promise<ReadonlyArray<CourseRosterEntry>>;
  readonly claimLiveCourseInvitation: (
    course: CourseInstanceReference,
  ) => Promise<{ readonly activeStudentMembership: boolean }>;
  readonly revokeLiveCourseRosterEntry: (
    course: CourseInstanceReference,
    rosterId: string,
  ) => Promise<void>;
}
