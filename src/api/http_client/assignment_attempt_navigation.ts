import type { AssignmentAttemptReference } from "../../../generated/api/AssignmentAttemptReference";
import type { ApiClient } from "../client";
import type {
  StudentAssignmentAttemptContext,
  StudentAssignmentAttemptNavigationClient,
  StudentAssignmentAttemptPresentation,
  StudentAssignmentAttemptResponseSaveAcknowledgement,
  StudentAssignmentAttemptSubmissionAcknowledgement,
} from "../assignment_attempt_navigation";
import {
  decodeStudentAssignmentAttemptContext,
  decodeStudentAssignmentAttemptPresentation,
  decodeStudentAssignmentAttemptProgress,
  decodeStudentAssignmentAttemptResponseSaveAcknowledgement,
  decodeStudentAssignmentAttemptSubmissionAcknowledgement,
} from "../decoders/assignment_attempt_navigation";
import { decodeStudentResponse } from "../decoders/question_delivery";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function attemptPath(value: AssignmentAttemptReference): string {
  if (!/^R-[1-9][0-9]{0,9}$/u.test(value) || Number(value.slice(2)) > 2_147_483_647)
    throw new ApiProtocolError("Assignment Attempt reference must be canonical");
  return `/api/assignment-attempts/${encodeURIComponent(value)}`;
}

function positionPathSegment(position: number): string {
  if (!Number.isSafeInteger(position) || position < 1 || position > 2_147_483_647)
    throw new ApiProtocolError("Assignment Attempt position must be a positive 31-bit integer");
  return String(position);
}

async function read<T>(
  fetcher: ApiFetch,
  basePath: string,
  path: string,
  decode: (value: unknown, path?: string) => T,
): Promise<T> {
  const response = await requestSameOrigin(fetcher, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200)
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  return decode(await boundedResponseJson(response, path), "response");
}

export function createStudentAssignmentAttemptNavigationClient(
  fetcher: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentAssignmentAttemptNavigationClient> {
  return {
    getStudentAssignmentAttemptContext: (attempt): Promise<StudentAssignmentAttemptContext> =>
      read(
        fetcher,
        basePath,
        `${attemptPath(attempt)}/context`,
        decodeStudentAssignmentAttemptContext,
      ).then((context) => {
        if (context.assignmentAttempt !== attempt)
          throw new ApiProtocolError("Assignment Attempt context does not match its request");
        return context;
      }),
    getStudentAssignmentAttemptProgress: (attempt) =>
      read(
        fetcher,
        basePath,
        `${attemptPath(attempt)}/student-progress`,
        decodeStudentAssignmentAttemptProgress,
      ).then((progress) => {
        if (progress.assignmentAttempt !== attempt)
          throw new ApiProtocolError("Assignment Attempt progress does not match its request");
        return progress;
      }),
    getStudentAssignmentAttemptPresentation: (
      attempt,
      position,
    ): Promise<StudentAssignmentAttemptPresentation> => {
      const positionPath = positionPathSegment(position);
      return read(
        fetcher,
        basePath,
        `${attemptPath(attempt)}/student-question?position=${positionPath}`,
        decodeStudentAssignmentAttemptPresentation,
      ).then((presentation) => {
        if (presentation.position !== position)
          throw new ApiProtocolError(
            "Assignment Attempt presentation position does not match its request",
          );
        return presentation;
      });
    },
    saveStudentAssignmentAttemptResponse: async (
      attempt,
      position,
      response,
    ): Promise<StudentAssignmentAttemptResponseSaveAcknowledgement> => {
      const path = `${attemptPath(attempt)}/responses/${positionPathSegment(position)}`;
      const request = decodeStudentResponse(response, "request.response");
      const result = await requestSameOrigin(fetcher, basePath, path, {
        method: "PUT",
        body: { response: request },
      });
      requireNoStore(result, path);
      if (!result.ok) throw new ApiRequestError(result.status, path);
      if (result.status !== 200)
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      const acknowledgement = decodeStudentAssignmentAttemptResponseSaveAcknowledgement(
        await boundedResponseJson(result, path),
        "response",
      );
      if (acknowledgement.assignmentAttempt !== attempt)
        throw new ApiProtocolError(
          "Saved response acknowledgement attempt does not match its request",
        );
      if (acknowledgement.position !== position)
        throw new ApiProtocolError(
          "Saved response acknowledgement position does not match its request",
        );
      return acknowledgement;
    },
    submitStudentAssignmentAttempt: async (
      attempt,
    ): Promise<StudentAssignmentAttemptSubmissionAcknowledgement> => {
      const path = `${attemptPath(attempt)}/submission`;
      const result = await requestSameOrigin(fetcher, basePath, path, { method: "POST" });
      requireNoStore(result, path);
      if (!result.ok) throw new ApiRequestError(result.status, path);
      if (result.status !== 200)
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      const acknowledgement = decodeStudentAssignmentAttemptSubmissionAcknowledgement(
        await boundedResponseJson(result, path),
        "response",
      );
      if (acknowledgement.assignmentAttempt !== attempt)
        throw new ApiProtocolError(
          "Assignment Attempt submission acknowledgement does not match its request",
        );
      return acknowledgement;
    },
  };
}
