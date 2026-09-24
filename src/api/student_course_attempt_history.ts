// Browser contract for cursor-paginated self-only Course Attempt History.

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { StudentCourseAttemptHistoryEntry as EntryContract } from "../../generated/api/StudentCourseAttemptHistoryEntry";
import type { StudentCourseAttemptHistoryPage as PageContract } from "../../generated/api/StudentCourseAttemptHistoryPage";

export type StudentCourseAttemptHistoryEntry = EntryContract;
export type StudentCourseAttemptHistoryPage = PageContract;

export interface StudentCourseAttemptHistoryClient {
  readonly listStudentCourseAttemptHistory: (
    courseInstanceId: CourseInstanceId,
    pageSize?: number,
    cursor?: string,
  ) => Promise<StudentCourseAttemptHistoryPage>;
}
