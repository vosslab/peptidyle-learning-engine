// model.ts - typed, browser-safe state helpers for assignment access modifiers.

import type { AssignmentPolicyPatchUpdateRequest } from "../../../generated/api/AssignmentPolicyPatchUpdateRequest";
import type { AccommodationPatchUpdateRequest } from "../../../generated/api/AccommodationPatchUpdateRequest";
import type { TeachingPreviewFieldSource } from "../../../generated/api/TeachingPreviewFieldSource";
import type { TeachingTimeFieldPatch } from "../../../generated/api/TeachingTimeFieldPatch";

export type ModifierScope = "accommodation";
export type PatchKind = "inherit" | "set" | "unrestricted";
export type ModifierMode = "extendOnly" | "override";

export interface ModifierPatchDraft {
  readonly availableAt: { readonly kind: PatchKind; readonly value: string };
  readonly dueAt: { readonly kind: PatchKind; readonly value: string };
  readonly closesAt: { readonly kind: PatchKind; readonly value: string };
  readonly timeLimitSeconds: { readonly kind: PatchKind; readonly value: string };
  readonly attemptLimit: { readonly kind: PatchKind; readonly value: string };
}

export interface PreviewSubject {
  readonly reference: string;
  readonly display: string;
}

/** A reload changes only the strong revision; a caller-owned modifier draft is deliberately retained. */
export function adoptReloadedRevision<T>(
  revision: string,
  draft: T,
): { readonly revision: string; readonly draft: T } {
  return { revision, draft };
}

export function emptyPatchDraft(): ModifierPatchDraft {
  return {
    availableAt: { kind: "inherit", value: "" },
    dueAt: { kind: "inherit", value: "" },
    closesAt: { kind: "inherit", value: "" },
    timeLimitSeconds: { kind: "inherit", value: "" },
    attemptLimit: { kind: "inherit", value: "" },
  };
}

/** Preserve the course wall-clock value; browser and machine time zones are irrelevant here. */
export function canonicalCourseLocalDateTime(value: string): string {
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
  return { kind: "set", value: canonicalCourseLocalDateTime(field.value) };
}

function limitPatch(
  field: ModifierPatchDraft["timeLimitSeconds"],
  label: string,
): { kind: "inherit" } | { kind: "set"; value: number } | { kind: "unrestricted" } {
  if (field.kind === "inherit") return { kind: "inherit" };
  if (field.kind === "unrestricted") return { kind: "unrestricted" };
  return { kind: "set", value: positiveInteger(field.value, label) };
}

export function policyRequest(
  mode: ModifierMode,
  draft: ModifierPatchDraft,
): AssignmentPolicyPatchUpdateRequest | AccommodationPatchUpdateRequest {
  return {
    mode,
    patch: {
      availableAt: timePatch(draft.availableAt),
      dueAt: timePatch(draft.dueAt),
      closesAt: timePatch(draft.closesAt),
      timeLimitSeconds: limitPatch(draft.timeLimitSeconds, "Whole-run seconds"),
      attemptLimit: limitPatch(draft.attemptLimit, "Attempt limit"),
    },
  };
}

export function sourceLabel(source: TeachingPreviewFieldSource): string {
  if (source.kind === "base") return source.label;
  return source.label;
}

export function startLabel(
  start: "mayStart" | "notYetAvailable" | "closed" | "attemptLimitReached" | "dueDateRejectsNewRun",
): string {
  const labels: Record<typeof start, string> = {
    mayStart: "May start",
    notYetAvailable: "Not yet available",
    closed: "Closed",
    attemptLimitReached: "Attempt limit reached",
    dueDateRejectsNewRun: "Due date prevents a new run",
  };
  return labels[start];
}
