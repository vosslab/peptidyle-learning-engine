// Same-origin transport for the current Student Course Landing projection.

import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type { LiveStudentCourseLandingClient } from "../live_student_course_landing";
import {
  decodeLiveStudentAssignmentLandings,
  decodeLiveStudentCourseInvitations,
  decodeLiveStudentCourseLandings,
} from "../decoders/live_student_course_landing";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function assignmentLandingPath(course: CourseInstanceReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || Number(course.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/assignment-landing`;
}

async function landingJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200) {
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  }
  return decoder(await boundedResponseJson(response, path), "response");
}

/** Composes only current-Student Course Landing reads, never a generic course listing. */
export function createLiveStudentCourseLandingClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveStudentCourseLandingClient> {
  return {
    listPendingLiveStudentCourseInvitations: () =>
      landingJson(
        fetchImplementation,
        basePath,
        "/api/student/course-invitations",
        decodeLiveStudentCourseInvitations,
      ),
    listLiveStudentCourses: () =>
      landingJson(
        fetchImplementation,
        basePath,
        "/api/student/course-instances",
        decodeLiveStudentCourseLandings,
      ),
    listLiveStudentAssignments: (course) =>
      landingJson(
        fetchImplementation,
        basePath,
        assignmentLandingPath(course),
        decodeLiveStudentAssignmentLandings,
      ),
  };
}
