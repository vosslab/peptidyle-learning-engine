// Strict same-origin transport for Student Assignment Access and start.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type {
  LiveAssignmentAccess,
  LiveAssignmentAttempt,
  LiveAssignmentAttemptIssuanceClient,
} from "../assignment_attempt_issuance";
import {
  decodeLiveAssignmentAccess,
  decodeLiveAssignmentAttempt,
} from "../decoders/assignment_attempt_issuance";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function assignmentPath(course: CourseInstanceReference, assignment: AssignmentReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || Number(course.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  if (!/^A-[1-9][0-9]{0,9}$/u.test(assignment) || Number(assignment.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Assignment reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/assignments/${encodeURIComponent(assignment)}`;
}

async function assignmentJson<T>(
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
export function createLiveAssignmentAttemptIssuanceClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveAssignmentAttemptIssuanceClient> {
  return {
    getLiveAssignmentAccess: (course, assignment): Promise<LiveAssignmentAccess> => {
      const path = `${assignmentPath(course, assignment)}/access`;
      return assignmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssignmentAccess,
        "GET",
        200,
      );
    },
    startLiveAssignment: (course, assignment): Promise<LiveAssignmentAttempt> => {
      const path = `${assignmentPath(course, assignment)}/start`;
      return assignmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssignmentAttempt,
        "POST",
        201,
      );
    },
  };
}
