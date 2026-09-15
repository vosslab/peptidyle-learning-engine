// assessment_workspace_load_model.ts - pure classification for the workspace authority boundary.

import { ApiRequestError } from "../../api/http_client";

export type AssessmentWorkspaceLoadState = "denied" | "unavailable" | "error";

/** Keep expected authority/resource outcomes separate from recoverable load failures. */
export function assessmentWorkspaceLoadFailureState(error: unknown): AssessmentWorkspaceLoadState {
  if (error instanceof ApiRequestError) {
    if (error.status === 401 || error.status === 403) return "denied";
    if (error.status === 404) return "unavailable";
  }
  return "error";
}
