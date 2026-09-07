import type { CourseRosterEntry } from "./course_roster";

export interface SupportCapabilityClient {
  readonly readSupportCourseRoster: (capabilityId: string) => Promise<ReadonlyArray<CourseRosterEntry>>;
}
