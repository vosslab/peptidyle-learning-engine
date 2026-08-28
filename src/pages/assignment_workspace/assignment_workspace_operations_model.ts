// assignment_workspace_operations_model.ts - pure Instructor recovery-page state and safe wording.

import type {
  GradingOperationActionId,
  GradingOperationGroupBy,
  GradingOperationStrongEtag,
  InstructorGradingOperationRow,
} from "../../api/decoders/grading_operations";
import { ApiRequestError } from "../../api/http_client";
import type { GradingOperationReference } from "../../../generated/api/GradingOperationReference";

export type GradingOperationsActionIntent =
  | {
      readonly kind: "retry";
      readonly operation: GradingOperationReference;
      readonly expectedRevision: GradingOperationStrongEtag;
      readonly idempotencyKey: GradingOperationActionId;
    }
  | {
      readonly kind: "recalculate";
      readonly expectedRevision: GradingOperationStrongEtag;
      readonly idempotencyKey: GradingOperationActionId;
    };

export type GradingOperationsActionFailure =
  | { readonly kind: "stale"; readonly message: string }
  | { readonly kind: "retryable"; readonly message: string };

export interface GradingOperationsListPosition {
  readonly groupBy: GradingOperationGroupBy;
  readonly cursor: string | undefined;
}

/** A grouping change begins a new ordered list; cursors never cross that boundary. */
export function gradingOperationsPositionForGroup(
  groupBy: GradingOperationGroupBy,
): GradingOperationsListPosition {
  return { groupBy, cursor: undefined };
}

/** The same accepted intent is deliberately replayed after an ambiguous transport outcome. */
export function retryGradingOperationsAction(
  intent: GradingOperationsActionIntent,
): GradingOperationsActionIntent {
  return intent;
}

export function retryOperationIntent(
  operation: GradingOperationReference,
  revision: number,
  idempotencyKey: GradingOperationActionId,
): GradingOperationsActionIntent {
  return {
    kind: "retry",
    operation,
    expectedRevision: `"${revision}"`,
    idempotencyKey,
  };
}

export function recalculationIntent(
  expectedRevision: GradingOperationStrongEtag,
  idempotencyKey: GradingOperationActionId,
): GradingOperationsActionIntent {
  return { kind: "recalculate", expectedRevision, idempotencyKey };
}

/** Conflict responses require an explicit fresh assignment read before another request. */
export function gradingOperationsActionFailure(error: unknown): GradingOperationsActionFailure {
  if (error instanceof ApiRequestError && [409, 412, 428].includes(error.status)) {
    return {
      kind: "stale",
      message:
        "This assignment changed before the grading request was accepted. Reload the latest assignment before continuing.",
    };
  }
  return {
    kind: "retryable",
    message:
      "We could not confirm that grading received this request. Try the same request again to check its accepted status.",
  };
}

export function gradingOperationsGroupLabel(row: InstructorGradingOperationRow): string {
  switch (row.group.kind) {
    case "question":
      return `Question: ${row.group.title}`;
    case "learner":
      return `Learner: ${row.group.displayName}`;
    case "assignment":
      return "Entire assignment";
  }
}

export function gradingOperationsReasonLabel(row: InstructorGradingOperationRow): string {
  switch (row.operation.reason) {
    case "grader_contract_failure":
      return "Automatic grading needs review before it can continue.";
    case "grader_execution_failure":
      return "Automatic grading stopped before it could finish.";
    case "issued_evidence_integrity":
      return "The submitted work record needs instructor attention.";
    case "retry_exhausted":
      return "Automatic grading needs another instructor-approved attempt.";
    case "scoring_recalculation_requested":
      return "Current grades are being refreshed.";
    case "instructor_requested_recalculation":
      return "You requested a grade refresh for this assignment.";
    case "scoring_recalculation_failed":
      return "The grade refresh needs instructor attention.";
  }
}

export function gradingOperationsStateLabel(row: InstructorGradingOperationRow): string {
  switch (row.operation.state) {
    case "actionable":
      return "Ready for instructor action";
    case "action_in_progress":
      return "Request accepted; grading is in progress";
    case "completed":
      return "Completed";
    case "repair_required":
      return "Needs instructor review";
    case "failed":
      return "Needs a new instructor decision";
    case "superseded":
      return "Replaced by a newer grading request";
  }
}

export function gradingOperationsAffectedLearnersLabel(count: number): string {
  return count === 1 ? "1 affected learner" : `${count} affected learners`;
}

export function gradingOperationsTrustGenerationLabel(row: InstructorGradingOperationRow): string {
  const label = row.trustGeneration.kind === "execution" ? "Attempt grading" : "Assignment grading";
  return `${label} generation ${row.trustGeneration.generation}`;
}

export function gradingOperationsRetryLabel(row: InstructorGradingOperationRow): string {
  return `Retry grading operation ${row.operation.reference}`;
}
