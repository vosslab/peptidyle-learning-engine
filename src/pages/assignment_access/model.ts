// model.ts - typed, browser-safe state helpers for assignment access modifiers.

import type { HypotheticalStudentViewScenarioModifiers } from "../../../generated/api/HypotheticalStudentViewScenarioModifiers";
import type { TeachingTimeFieldPatch } from "../../../generated/api/TeachingTimeFieldPatch";

export type PatchKind = "inherit" | "set" | "unrestricted";
export type ModifierMode = "extend_only" | "replace";

export interface ModifierPatchDraft {
  readonly availableAt: { readonly kind: PatchKind; readonly value: string };
  readonly dueAt: { readonly kind: PatchKind; readonly value: string };
  readonly closesAt: { readonly kind: PatchKind; readonly value: string };
  readonly assignmentAttemptTimeLimitSeconds: { readonly kind: PatchKind; readonly value: string };
  readonly attemptLimit: { readonly kind: PatchKind; readonly value: string };
}

export function emptyPatchDraft(): ModifierPatchDraft {
  return {
    availableAt: { kind: "inherit", value: "" },
    dueAt: { kind: "inherit", value: "" },
    closesAt: { kind: "inherit", value: "" },
    assignmentAttemptTimeLimitSeconds: { kind: "inherit", value: "" },
    attemptLimit: { kind: "inherit", value: "" },
  };
}

/** Preserve the course wall-clock value; browser and machine time zones are irrelevant here. */
export function canonicalCourseLocalDateAndTime(value: string): string {
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/u.test(value)) return `${value}:00.000`;
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/u.test(value)) return `${value}.000`;
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(value)) return value;
  throw new Error("Enter a complete course-local date and time.");
}

function positiveInteger(value: string, label: string): number {
  if (!/^[1-9][0-9]*$/u.test(value)) throw new Error(`${label} must be a positive whole number.`);
  const result = Number(value);
  if (!Number.isSafeInteger(result)) throw new Error(`${label} is too large.`);
  return result;
}

function timePatch(field: ModifierPatchDraft["availableAt"]): TeachingTimeFieldPatch {
  if (field.kind === "inherit") return { kind: "inherit" };
  if (field.kind === "unrestricted") return { kind: "unrestricted" };
  return { kind: "set", value: canonicalCourseLocalDateAndTime(field.value) };
}

function limitPatch(
  field: ModifierPatchDraft["assignmentAttemptTimeLimitSeconds"],
  label: string,
): { kind: "inherit" } | { kind: "set"; value: number } | { kind: "unrestricted" } {
  if (field.kind === "inherit") return { kind: "inherit" };
  if (field.kind === "unrestricted") return { kind: "unrestricted" };
  return { kind: "set", value: positiveInteger(field.value, label) };
}

export function policyRequest(
  mode: ModifierMode,
  draft: ModifierPatchDraft,
): HypotheticalStudentViewScenarioModifiers {
  return {
    mode,
    adjustment: {
      available_at: timePatch(draft.availableAt),
      due_at: timePatch(draft.dueAt),
      closes_at: timePatch(draft.closesAt),
      assignment_attempt_time_limit_seconds: limitPatch(
        draft.assignmentAttemptTimeLimitSeconds,
        "Whole Assignment Attempt seconds",
      ),
      attempt_limit: limitPatch(draft.attemptLimit, "Attempt limit"),
    },
  };
}
