// Same-origin transport for Question display-duration checkpoints.

import type { AssessmentAttemptId } from "../../../generated/api/AssessmentAttemptId";
import type { ApiClient } from "../client";
import type {
  StudentQuestionDisplayDurationCheckpoint,
  StudentQuestionDisplayDurationClient,
} from "../student_question_display_duration";
import { decodeStudentQuestionDisplayDurationCheckpoint } from "../decoders/student_question_display_duration";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function checkpointPath(assessmentAttemptId: AssessmentAttemptId, position: number): string {
  if (parseAssessmentAttemptId(assessmentAttemptId) === null) {
    throw new ApiProtocolError("Assessment Attempt ID must be canonical");
  }
  if (!Number.isSafeInteger(position) || position < 1) {
    throw new ApiProtocolError("Question position must be a positive safe integer");
  }
  return `/api/assessment-attempts/${encodeURIComponent(assessmentAttemptId)}/questions/${position}/display-duration`;
}

export function createStudentQuestionDisplayDurationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentQuestionDisplayDurationClient> {
  return {
    checkpointStudentQuestionDisplayDuration: async (
      assessmentAttemptId,
      position,
      cumulativeDisplayDurationMs,
    ): Promise<StudentQuestionDisplayDurationCheckpoint> => {
      if (!Number.isSafeInteger(cumulativeDisplayDurationMs) || cumulativeDisplayDurationMs < 0) {
        throw new ApiProtocolError("Question display duration must be a nonnegative safe integer");
      }
      const path = checkpointPath(assessmentAttemptId, position);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        body: { cumulativeDisplayDurationMs },
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200)
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      const checkpoint = decodeStudentQuestionDisplayDurationCheckpoint(
        await boundedResponseJson(response, path),
      );
      if (
        checkpoint.assessmentAttemptId !== assessmentAttemptId ||
        checkpoint.position !== position ||
        checkpoint.cumulativeDisplayDurationMs < cumulativeDisplayDurationMs
      ) {
        throw new ApiProtocolError(
          "Question display-duration checkpoint does not match its request",
        );
      }
      return checkpoint;
    },
  };
}
