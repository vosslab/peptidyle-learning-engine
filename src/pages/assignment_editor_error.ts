// assignment_editor_error.ts - visible recovery policy for ordinary assignment-definition actions.

import type { AssignmentCapabilityViolation } from "../api/contracts";
import {
  AssignmentConflictError,
  AssignmentValidationError,
  ApiRequestError,
} from "../api/http_client";

export type AssignmentEditorErrorResolution =
  | { readonly kind: "conflict"; readonly message: string }
  | {
      readonly kind: "validation";
      readonly message: string;
      readonly violations: ReadonlyArray<AssignmentCapabilityViolation>;
    }
  | { readonly kind: "other"; readonly message: string };

export function resolveAssignmentEditorError(
  error: unknown,
  fallback: string,
): AssignmentEditorErrorResolution {
  if (
    error instanceof AssignmentConflictError ||
    (error instanceof ApiRequestError && error.status === 409)
  ) {
    return {
      kind: "conflict",
      message:
        "A newer assignment revision or issued learner work prevents this structural save. Your edits are still here. Reload to compare revisions; when learner work has started, create a new assignment or use the supported future-run replacement workflow.",
    };
  }
  if (error instanceof AssignmentValidationError) {
    return {
      kind: "validation",
      message: "The assignment settings need adjustment before they can be saved.",
      violations: error.violations,
    };
  }
  return {
    kind: "other",
    message: error instanceof Error ? `${error.message} ${fallback}` : fallback,
  };
}
