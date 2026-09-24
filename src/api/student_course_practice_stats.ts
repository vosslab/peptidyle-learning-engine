// Browser contract for self-only exact-revision Student Course Response Stats.

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { StudentCourseResponseStats as Contract } from "../../generated/api/StudentCourseResponseStats";

export type StudentCourseResponseStats = Contract;

export interface StudentCourseResponseStatsClient {
  readonly getStudentCourseResponseStats: (
    courseInstanceId: CourseInstanceId,
  ) => Promise<StudentCourseResponseStats>;
}
