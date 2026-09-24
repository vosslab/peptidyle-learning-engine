// Browser contract for self-only exact-revision Student Course Practice Stats.

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { StudentCoursePracticeStats as Contract } from "../../generated/api/StudentCoursePracticeStats";

export type StudentCoursePracticeStats = Contract;

export interface StudentCoursePracticeStatsClient {
  readonly getStudentCoursePracticeStats: (
    courseInstanceId: CourseInstanceId,
  ) => Promise<StudentCoursePracticeStats>;
}
