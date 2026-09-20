// Strict same-origin transport for one Student-owned completed Assessment Attempt.

import type { AssessmentAttemptId } from "../../../generated/api/AssessmentAttemptId";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import type { ApiClient } from "../client";
import type {
  StudentAssessmentAttemptHistory,
  StudentAssessmentAttemptHistoryClient,
} from "../assessment_attempt_history";
import { decodeStudentAssessmentAttemptHistory } from "../decoders/assessment_attempt_history";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function historyPath(assessmentAttemptId: AssessmentAttemptId): string {
  if (parseAssessmentAttemptId(assessmentAttemptId) === null) {
    throw new ApiProtocolError("Assessment Attempt ID must be a UUID");
  }
  return `/api/assessment-attempts/${encodeURIComponent(assessmentAttemptId)}/history`;
}

async function getHistory(
  fetchImplementation: ApiFetch,
  basePath: string,
  assessmentAttemptId: AssessmentAttemptId,
): Promise<StudentAssessmentAttemptHistory> {
  const path = historyPath(assessmentAttemptId);
  // ASVS 3.5.1 and 4.1.1: retain the shared same-origin, JSON-only request boundary.
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200)
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  // ASVS 1.5.2: bounded JSON enters the browser only through the closed history decoder.
  const history = decodeStudentAssessmentAttemptHistory(
    await boundedResponseJson(response, path),
    "response",
  );
  if (history.assessmentAttemptId !== assessmentAttemptId) {
    throw new ApiProtocolError("Assessment Attempt history does not match its request");
  }
  return history;
}

/** Composes the selected-history read separately from active Attempt navigation. */
export function createStudentAssessmentAttemptHistoryClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentAssessmentAttemptHistoryClient> {
  return {
    getStudentAssessmentAttemptHistory: (assessmentAttemptId) =>
      getHistory(fetchImplementation, basePath, assessmentAttemptId),
  };
}
