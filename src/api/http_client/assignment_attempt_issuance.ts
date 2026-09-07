// Strict same-origin transport for M11 Student Assignment Access and start.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type { LiveAssignmentAttemptIssuanceClient } from "../assignment_attempt_issuance";
import {
  decodeLiveAssignmentAccess,
  decodeLiveAssignmentAttempt,
  decodeLiveNativePleSubmissionAcknowledgement,
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

function presentationNoncePathSegment(presentationNonce: string): string {
  if (!/^[0-9a-f]{32}$/u.test(presentationNonce)) {
    throw new ApiProtocolError("Presentation nonce must be canonical");
  }
  return encodeURIComponent(presentationNonce);
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

/** Composes M11 separately from generic, UUID-based attempt clients. */
export function createLiveAssignmentAttemptIssuanceClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveAssignmentAttemptIssuanceClient> {
  return {
    getLiveAssignmentAccess: (course, assignment) => {
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
    startLiveAssignment: (course, assignment) => {
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
    submitLiveNativePleResponse: async (course, assignment, presentationNonce, response) => {
      const path = `${assignmentPath(course, assignment)}/presentations/${presentationNoncePathSegment(presentationNonce)}/submissions`;
      const result = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ response }),
      });
      requireNoStore(result, path);
      if (!result.ok) throw new ApiRequestError(result.status, path);
      if (result.status !== 201) {
        throw new ApiProtocolError(`API response ${path} must use status 201`);
      }
      const acknowledgement = decodeLiveNativePleSubmissionAcknowledgement(
        await boundedResponseJson(result, path),
        "response",
      );
      if (acknowledgement.presentationNonce !== presentationNonce) {
        throw new ApiProtocolError("Submission acknowledgement nonce does not match its request");
      }
      return acknowledgement;
    },
  };
}
