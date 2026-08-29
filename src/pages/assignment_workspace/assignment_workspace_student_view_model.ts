// Pure state and copy helpers for the Instructor Student-view composition.

import { ApiRequestError } from "../../api/http_client";
import type { StudentAssignmentPresentationData } from "../../components/student_assignment_presentation";

export const STUDENT_VIEW_CUE =
  "Student view - current live assignment. Use Student entry to submit graded work.";
export const STUDENT_VIEW_ENTRY_PATH = "/sign-in";

export type StudentViewState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly assignment: StudentAssignmentPresentationData }
  | { readonly kind: "unavailable" }
  | { readonly kind: "error" };

/** Keep authorization and transport failures non-enumerating in the page state. */
export function studentViewFailureState(error: unknown): "unavailable" | "error" {
  if (error instanceof ApiRequestError && [401, 403, 404].includes(error.status)) {
    return "unavailable";
  }
  return "error";
}
