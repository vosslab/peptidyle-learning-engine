// Same-origin transport for the current Student Course Landing projection.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type { LiveStudentCourseLandingClient } from "../live_student_course_landing";
import {
  decodeLiveStudentAssessmentLandings,
  decodeLiveStudentCourseInvitations,
  decodeLiveStudentCourseLandings,
  decodeStudentCourseActiveAttempt,
  decodeStudentCourseProgress,
  decodeStudentLatestFeedback,
} from "../decoders/live_student_course_landing";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch, type RequestOptions } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceId } from "../../navigation/public_route";

function assessmentLandingPath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}/assessment-landing`;
}

function progressPath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/student/course-instances/${encodeURIComponent(courseInstanceId)}/progress`;
}

function activeAttemptPath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/student/course-instances/${encodeURIComponent(courseInstanceId)}/active-attempt`;
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
    listLiveStudentAssessments: (courseInstanceId) =>
      landingJson(
        fetchImplementation,
        basePath,
        assessmentLandingPath(courseInstanceId),
        decodeLiveStudentAssessmentLandings,
      ),
    getStudentCourseProgress: (courseInstanceId) =>
      landingJson(
        fetchImplementation,
        basePath,
        progressPath(courseInstanceId),
        decodeStudentCourseProgress,
      ),
    getStudentCourseActiveAttempt: (courseInstanceId) =>
      landingJson(
        fetchImplementation,
        basePath,
        activeAttemptPath(courseInstanceId),
        decodeStudentCourseActiveAttempt,
      ),
    getStudentLatestFeedback: () =>
      landingJson(
        fetchImplementation,
        basePath,
        "/api/student/latest-feedback",
        decodeStudentLatestFeedback,
      ),
  };
}
