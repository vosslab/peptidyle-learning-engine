import type { CourseGroupPurpose } from "../../../generated/api/CourseGroupPurpose";
import type { CourseGroupMembershipWarningView } from "../../../generated/api/CourseGroupMembershipWarningView";
import type { CourseGroupSummaryView } from "../../../generated/api/CourseGroupSummaryView";
import type { RetentionStateView } from "../../../generated/api/RetentionStateView";

export const COURSE_GROUP_PURPOSES: ReadonlyArray<CourseGroupPurpose> = [
  "section",
  "lab",
  "cohort",
  "accommodation",
  "work",
];

export function purposeLabel(purpose: CourseGroupPurpose): string {
  switch (purpose) {
    case "section":
      return "Section";
    case "lab":
      return "Lab";
    case "cohort":
      return "Cohort";
    case "accommodation":
      return "Accommodation";
    case "work":
      return "Work";
  }
}

export function policyCopy(policy: "allow" | "warn"): string {
  return policy === "warn"
    ? "Warn when a learner belongs to more than one group of this purpose; the warning never blocks a valid write."
    : "Allow learners to belong to more than one group of this purpose.";
}

export function retentionStateCopy(state: RetentionStateView): string {
  switch (state) {
    case "active":
      return "Student records are active.";
    case "notificationDue":
      return "A retention decision is due.";
    case "studentRecordsArchived":
      return "Student records are archived from ordinary learner access.";
    case "studentRecordsDeleted":
      return "Student records have been permanently deleted.";
  }
}

export function groupConflictCopy(): string {
  return "The group changed elsewhere. Your draft is preserved; reload the latest group before retrying.";
}

export function referencedGroupCopy(): string {
  return "This group is still referenced by an assignment audience or policy modifier and cannot be deleted.";
}

/** The server warning count aggregates every purpose with a warn policy. */
export function membershipWarningCopy(warning: CourseGroupMembershipWarningView): string {
  if (warning.disposition === "allowed") {
    return "Course-group membership check: allowed. No overlapping memberships need attention.";
  }
  const count = warning.warningCount;
  const noun = count === 1 ? "overlapping membership" : "overlapping memberships";
  return `Course-group membership check: allowed with warning. ${count} ${noun} need attention.`;
}

/** Preserves existing group rows when a cursor page overlaps a prior response. */
export function appendGroupPage(
  current: ReadonlyArray<CourseGroupSummaryView>,
  next: ReadonlyArray<CourseGroupSummaryView>,
): ReadonlyArray<CourseGroupSummaryView> {
  const references = new Set<string>();
  const merged: Array<CourseGroupSummaryView> = [];
  for (const group of current) {
    references.add(group.reference);
    merged.push(group);
  }
  for (const group of next) {
    if (!references.has(group.reference)) {
      references.add(group.reference);
      merged.push(group);
    }
  }
  return merged;
}
