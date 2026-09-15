// assessment_workspace_policy_model.ts - policy-page request construction and local control values.

import type { SaveLiveAssessmentInput } from "../../api/assessment_release";

/** The teaching default applied when an Instructor chooses a new due date. */
export const DEFAULT_DUE_TIME = "23:59:00.000";

/** Replaces policy-owned values while carrying current Entries and access bounds unchanged. */
export function assessmentPolicySaveInput(
  current: SaveLiveAssessmentInput,
  replacement: Pick<
    SaveLiveAssessmentInput,
    | "instructions"
    | "dueAt"
    | "lateWorkRule"
    | "assessmentAttemptTimeLimitSeconds"
    | "attemptLimit"
    | "activityRules"
    | "studentFeedbackReleaseRule"
  >,
): SaveLiveAssessmentInput {
  return {
    ...current,
    ...replacement,
    entries: current.entries,
    availableAt: current.availableAt,
    closesAt: current.closesAt,
  };
}

/** Converts a native local-date-time control value to the explicit wire form. */
export function canonicalLocalDateAndTime(value: string): string | null {
  if (value === "") return null;
  const match = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?$/u.exec(value);
  if (match === null) return null;
  const [, dateAndMinute, seconds = "00", fraction = ""] = match;
  return `${dateAndMinute}:${seconds}.${fraction.padEnd(3, "0")}`;
}

/** Returns the date portion of a stored local deadline for the date control. */
export function dueDateDraft(value: string | null): string {
  return value === null ? "" : value.slice(0, 10);
}

/** Returns the authored time or the teaching default for a newly selected date. */
export function dueTimeDraft(value: string | null): string {
  return value === null ? DEFAULT_DUE_TIME : value.slice(11);
}

/** Combines local date and time controls without converting them to an instant. */
export function localDueDateAndTime(date: string, time: string): string {
  return date === "" ? "" : `${date}T${time}`;
}

export type PositiveIntegerDraft = {
  readonly raw: string;
  readonly value: number | null;
  readonly valid: boolean;
};

/**
 * Parses an optional positive whole-number control.
 *
 * An empty control intentionally maps to null so the focused policy save can
 * clear an existing server-side time or attempt limit. Invalid text stays
 * local and never replaces the last valid payload value.
 */
export function optionalPositiveIntegerDraft(raw: string): PositiveIntegerDraft {
  if (raw === "") return { raw, value: null, valid: true };
  if (!/^[1-9][0-9]*$/u.test(raw)) return { raw, value: null, valid: false };
  const value = Number(raw);
  return Number.isSafeInteger(value) && value <= 2_147_483_647
    ? { raw, value, valid: true }
    : { raw, value: null, valid: false };
}
