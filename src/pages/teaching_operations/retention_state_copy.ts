import type { RetentionStateView } from "../../../generated/api/RetentionStateView";

/** Human-facing state copy for the independently owned retention panel. */
export function retentionStateCopy(state: RetentionStateView): string {
  switch (state) {
    case "active":
      return "Student records are active.";
    case "notificationDue":
      return "A retention decision is due.";
    case "studentRecordsArchived":
      return "Student records are archived from ordinary Student access.";
    case "studentRecordsDeleted":
      return "Student records have been permanently deleted.";
  }
}
