// Same-origin transport for self-only Student Course Practice Stats.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type {
  StudentCoursePracticeStats,
  StudentCoursePracticeStatsClient,
} from "../student_course_practice_stats";
import { decodeStudentCoursePracticeStats } from "../decoders/student_course_practice_stats";
import { parseCourseInstanceId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function statsPath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/student/course-instances/${encodeURIComponent(courseInstanceId)}/practice-stats`;
}

export function createStudentCoursePracticeStatsClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentCoursePracticeStatsClient> {
  return {
    getStudentCoursePracticeStats: async (
      courseInstanceId,
    ): Promise<StudentCoursePracticeStats> => {
      const path = statsPath(courseInstanceId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeStudentCoursePracticeStats(await boundedResponseJson(response, path));
    },
  };
}
