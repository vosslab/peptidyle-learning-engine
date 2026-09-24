// Same-origin transport for cursor-paginated Student Course Attempt History.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type {
  StudentCourseAttemptHistoryClient,
  StudentCourseAttemptHistoryPage,
} from "../student_course_attempt_history";
import { decodeStudentCourseAttemptHistoryPage } from "../decoders/student_course_attempt_history";
import { parseCourseInstanceId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function historyPath(
  courseInstanceId: CourseInstanceId,
  pageSize: number,
  cursor?: string,
): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > 100) {
    throw new ApiProtocolError("Attempt History page size must be between 1 and 100");
  }
  if (cursor !== undefined && (cursor.length === 0 || cursor.length > 512)) {
    throw new ApiProtocolError("Attempt History cursor is invalid");
  }
  const query = new URLSearchParams({ pageSize: String(pageSize) });
  if (cursor !== undefined) query.set("cursor", cursor);
  return `/api/student/course-instances/${encodeURIComponent(courseInstanceId)}/assessment-attempts?${query}`;
}

export function createStudentCourseAttemptHistoryClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentCourseAttemptHistoryClient> {
  return {
    listStudentCourseAttemptHistory: async (
      courseInstanceId,
      pageSize = 50,
      cursor,
    ): Promise<StudentCourseAttemptHistoryPage> => {
      const path = historyPath(courseInstanceId, pageSize, cursor);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeStudentCourseAttemptHistoryPage(
        await boundedResponseJson(response, path),
        "response",
      );
    },
  };
}
