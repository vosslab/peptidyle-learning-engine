// Same-origin transport for the current Student Course Landing projection.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type { LiveStudentCourseLandingClient } from "../live_student_course_landing";
import {
  decodeLiveStudentAssessmentLandings,
  decodeLiveStudentCourseInvitations,
  decodeLiveStudentCourseLandings,
} from "../decoders/live_student_course_landing";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch, type RequestOptions } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceId } from "../../navigation/public_route";

function assessmentLandingPath(course: CourseInstanceId): string {
  if (parseCourseInstanceId(course) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/assessment-landing`;
}

async function landingJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: RequestOptions = {},
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, options);
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
    listLiveStudentAssessments: (course) =>
      landingJson(
        fetchImplementation,
        basePath,
        assessmentLandingPath(course),
        decodeLiveStudentAssessmentLandings,
      ),
  };
}
