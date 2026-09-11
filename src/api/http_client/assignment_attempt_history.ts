// Strict same-origin transport for one Student-owned completed Assignment Attempt.

import type { AssignmentAttemptReference } from "../../../generated/api/AssignmentAttemptReference";
import type { ApiClient } from "../client";
import type {
  StudentAssignmentAttemptHistory,
  StudentAssignmentAttemptHistoryClient,
} from "../assignment_attempt_history";
import { decodeStudentAssignmentAttemptHistory } from "../decoders/assignment_attempt_history";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function historyPath(assignmentAttempt: AssignmentAttemptReference): string {
  // ASVS 2.2.1: accept only the closed public Attempt-reference grammar before transport.
  if (
    !/^R-[1-9][0-9]{0,9}$/u.test(assignmentAttempt) ||
    Number(assignmentAttempt.slice(2)) > 2_147_483_647
  ) {
    throw new ApiProtocolError("Assignment Attempt reference must be canonical");
  }
  return `/api/assignment-attempts/${encodeURIComponent(assignmentAttempt)}/history`;
}

async function getHistory(
  fetchImplementation: ApiFetch,
  basePath: string,
  assignmentAttempt: AssignmentAttemptReference,
): Promise<StudentAssignmentAttemptHistory> {
  const path = historyPath(assignmentAttempt);
  // ASVS 3.5.1 and 4.1.1: retain the shared same-origin, JSON-only request boundary.
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200)
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  // ASVS 1.5.2: bounded JSON enters the browser only through the closed history decoder.
  const history = decodeStudentAssignmentAttemptHistory(
    await boundedResponseJson(response, path),
    "response",
  );
  if (history.assignmentAttempt !== assignmentAttempt) {
    throw new ApiProtocolError("Assignment Attempt history does not match its request");
  }
  return history;
}

/** Composes the selected-history read separately from active Attempt navigation. */
export function createStudentAssignmentAttemptHistoryClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentAssignmentAttemptHistoryClient> {
  return {
    getStudentAssignmentAttemptHistory: (assignmentAttempt) =>
      getHistory(fetchImplementation, basePath, assignmentAttempt),
  };
}
