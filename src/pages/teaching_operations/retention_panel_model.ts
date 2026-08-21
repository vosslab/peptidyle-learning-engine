import type { RetentionActionOutcomeView } from "../../../generated/api/RetentionActionOutcomeView";
import type { RetentionStateView } from "../../../generated/api/RetentionStateView";

export interface RetentionActionAvailability {
  readonly archive: boolean;
  readonly delete: boolean;
  readonly extend: boolean;
}

/** The browser only offers mutations compatible with the server's closed projection. */
export function retentionActionAvailability(
  state: RetentionStateView,
): RetentionActionAvailability {
  switch (state) {
    case "active":
    case "notificationDue":
      return { archive: true, delete: true, extend: true };
    case "studentRecordsArchived":
    case "studentRecordsDeleted":
      return { archive: false, delete: false, extend: false };
  }
}

export function retentionOutcomeCopy(outcome: RetentionActionOutcomeView): string {
  switch (outcome) {
    case "scheduled":
      return "The retention action is scheduled.";
    case "inProgress":
      return "The retention action is already in progress.";
    case "completed":
      return "The retention action is complete.";
  }
}

export function retentionFailureCopy(status: number | undefined): string {
  if (status === 403) return "You do not have permission to change this course's retention.";
  if (status === 412)
    return "Retention changed elsewhere. Reload the latest retention state before retrying.";
  if (status === undefined)
    return "Retention is unavailable while offline. Reconnect and try again.";
  return "The retention action could not be completed. Try again.";
}

export function retentionReloadRequired(status: number | undefined): boolean {
  return status === 412;
}
