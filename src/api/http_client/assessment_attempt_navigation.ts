import type { AssessmentAttemptId } from "../../../generated/api/AssessmentAttemptId";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import type { ApiClient } from "../client";
import type {
  StudentAssessmentAttemptContext,
  StudentAssessmentAttemptNavigationClient,
  StudentAssessmentAttemptPresentation,
  StudentAssessmentAttemptResponseSaveAcknowledgement,
  StudentAssessmentAttemptSubmissionResult,
} from "../assessment_attempt_navigation";
import {
  decodeStudentAssessmentAttemptContext,
  decodeStudentAssessmentAttemptPresentation,
  decodeStudentAssessmentAttemptProgress,
  decodeStudentAssessmentAttemptResponseSaveAcknowledgement,
  decodeStudentAssessmentAttemptSubmissionResult,
} from "../decoders/assessment_attempt_navigation";
import { decodeStudentResponse } from "../decoders/question_delivery";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestPath, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function attemptPath(value: AssessmentAttemptId): string {
  if (parseAssessmentAttemptId(value) === null)
    throw new ApiProtocolError("Assessment Attempt ID must be a UUID");
  return `/api/assessment-attempts/${encodeURIComponent(value)}`;
}

function positionPathSegment(position: number): string {
  if (!Number.isSafeInteger(position) || position < 1 || position > 2_147_483_647)
    throw new ApiProtocolError("Assessment Attempt position must be a positive 31-bit integer");
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

export function createStudentAssessmentAttemptNavigationClient(
  fetcher: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentAssessmentAttemptNavigationClient> {
  return {
    studentAuthorContentDocumentUrl: (assessmentAttemptId, position): string => {
      // ASVS 1.2.2, 2.2.1: only an exact Attempt and issued position select this document.
      const path = `${attemptPath(assessmentAttemptId)}/questions/${positionPathSegment(position)}/author-content-document`;
      return requestPath(basePath, path);
    },
    getStudentAssessmentAttemptContext: (
      assessmentAttemptId,
    ): Promise<StudentAssessmentAttemptContext> =>
      read(
        fetcher,
        basePath,
        `${attemptPath(assessmentAttemptId)}/context`,
        decodeStudentAssessmentAttemptContext,
      ).then((context) => {
        if (context.assessmentAttemptId !== assessmentAttemptId) {
          throw new ApiProtocolError("Assessment Attempt context does not match its request");
        }
        return context;
      }),
    getStudentAssessmentAttemptProgress: (assessmentAttemptId) =>
      read(
        fetcher,
        basePath,
        `${attemptPath(assessmentAttemptId)}/student-progress`,
        decodeStudentAssessmentAttemptProgress,
      ).then((progress) => {
        if (progress.assessmentAttemptId !== assessmentAttemptId) {
          throw new ApiProtocolError("Assessment Attempt progress does not match its request");
        }
        return progress;
      }),
    getStudentAssessmentAttemptPresentation: (
      assessmentAttemptId,
      position,
    ): Promise<StudentAssessmentAttemptPresentation> => {
      const positionPath = positionPathSegment(position);
      return read(
        fetcher,
        basePath,
        `${attemptPath(assessmentAttemptId)}/student-question?position=${positionPath}`,
        decodeStudentAssessmentAttemptPresentation,
      ).then((presentation) => {
        if (presentation.position !== position)
          throw new ApiProtocolError(
            "Assessment Attempt presentation position does not match its request",
          );
        return presentation;
      });
    },
    saveStudentAssessmentAttemptResponse: async (
      assessmentAttemptId,
      position,
      response,
    ): Promise<StudentAssessmentAttemptResponseSaveAcknowledgement> => {
      const path = `${attemptPath(assessmentAttemptId)}/responses/${positionPathSegment(position)}`;
      const request = decodeStudentResponse(response, "request.response");
      const result = await requestSameOrigin(fetcher, basePath, path, {
        method: "PUT",
        body: { response: request },
      });
      requireNoStore(result, path);
      if (!result.ok) throw new ApiRequestError(result.status, path);
      if (result.status !== 200)
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      const acknowledgement = decodeStudentAssessmentAttemptResponseSaveAcknowledgement(
        await boundedResponseJson(result, path),
        "response",
      );
      if (acknowledgement.assessmentAttemptId !== assessmentAttemptId) {
        throw new ApiProtocolError(
          "Saved response acknowledgement attempt does not match its request",
        );
      }
      if (acknowledgement.position !== position) {
        throw new ApiProtocolError(
          "Saved response acknowledgement position does not match its request",
        );
      }
      return acknowledgement;
    },
    submitStudentAssessmentAttempt: async (
      assessmentAttemptId,
    ): Promise<StudentAssessmentAttemptSubmissionResult> => {
      const path = `${attemptPath(assessmentAttemptId)}/submission`;
      const result = await requestSameOrigin(fetcher, basePath, path, { method: "POST" });
      requireNoStore(result, path);
      if (!result.ok) throw new ApiRequestError(result.status, path);
      if (result.status !== 200)
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      const acknowledgement = decodeStudentAssessmentAttemptSubmissionResult(
        await boundedResponseJson(result, path),
        "response",
      );
      if (acknowledgement.assessmentAttemptId !== assessmentAttemptId) {
        throw new ApiProtocolError(
          "Assessment Attempt submission acknowledgement does not match its request",
        );
      }
      return acknowledgement;
    },
  };
}
