// Strict same-origin transport for Student Assessment Access and start.

import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type {
  LiveAssessmentAccess,
  LiveAssessmentAttempt,
  LiveAssessmentAttemptIssuanceClient,
} from "../assessment_attempt_issuance";
import {
  decodeLiveAssessmentAccess,
  decodeLiveAssessmentAttempt,
} from "../decoders/assessment_attempt_issuance";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseAssessmentId, parseCourseInstanceId } from "../../navigation/public_route";

function assessmentPath(course: CourseInstanceId, assessment: AssessmentId): string {
  if (parseCourseInstanceId(course) === null) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  if (parseAssessmentId(assessment) === null) {
    throw new ApiProtocolError("Assessment reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/assessments/${encodeURIComponent(assessment)}`;
}

async function assessmentJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  method: "GET" | "POST",
  status: 200 | 201,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, { method });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== status) {
    throw new ApiProtocolError(`API response ${path} must use status ${status}`);
  }
  return decoder(await boundedResponseJson(response, path), "response");
}

/** Composes this capability separately from generic, UUID-based attempt clients. */
export function createLiveAssessmentAttemptIssuanceClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveAssessmentAttemptIssuanceClient> {
  return {
    getLiveAssessmentAccess: (course, assessment): Promise<LiveAssessmentAccess> => {
      const path = `${assessmentPath(course, assessment)}/access`;
      return assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentAccess,
        "GET",
        200,
      );
    },
    startLiveAssessment: (course, assessment): Promise<LiveAssessmentAttempt> => {
      const path = `${assessmentPath(course, assessment)}/start`;
      return assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentAttempt,
        "POST",
        201,
      );
    },
  };
}
