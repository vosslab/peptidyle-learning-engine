// assignment_workspace_policy_model.ts - policy-page request construction and local control values.

import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { AssignmentPoliciesInput } from "../../api/contracts";

export type AssignmentPolicyFeedback =
  | { readonly kind: "success"; readonly message: string }
  | { readonly kind: "info"; readonly message: string }
  | { readonly kind: "error"; readonly message: string }
  | { readonly kind: "conflict"; readonly message: string };

export function assignmentPolicyFeedbackRole(
  feedback: AssignmentPolicyFeedback | undefined,
): "alert" | "status" {
  return feedback?.kind === "error" || feedback?.kind === "conflict" ? "alert" : "status";
}

export function assignmentPolicyCanReload(feedback: AssignmentPolicyFeedback | undefined): boolean {
  return feedback?.kind === "conflict";
}

/** Keeps the page's focused write closed to the policies-owned aggregate slice. */
export function assignmentPoliciesInput(
  audience: AssignmentPoliciesInput["audience"],
  disclosurePolicy: AssignmentPoliciesInput["disclosurePolicy"],
  policies: AssignmentPoliciesInput["policies"],
  teachingSettings: InstructorAssignmentTeachingSettingsLocal,
): AssignmentPoliciesInput {
  return { audience, disclosurePolicy, policies, teachingSettings };
}

/** Converts a native local-date-time control value to the explicit wire form. */
export function canonicalCourseLocalTime(value: string): string | null {
  if (value === "") return null;
  const normalized = value.length === 16 ? `${value}:00.000` : value;
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(normalized) ? normalized : null;
}

/** An empty group audience is incomplete before it crosses the strict server boundary. */
export function hasEmptyGroupAudience(audience: AssignmentPoliciesInput["audience"]): boolean {
  return audience.kind === "anyOfGroups" && audience.groups.length === 0;
}

export type PositiveIntegerDraft = {
  readonly raw: string;
  readonly value: number | null;
  readonly valid: boolean;
};

/** Keeps native number-control text visible while separating valid payload state. */
export function positiveIntegerDraft(raw: string): PositiveIntegerDraft {
  if (raw === "") return { raw, value: null, valid: true };
  if (!/^[1-9][0-9]*$/u.test(raw)) return { raw, value: null, valid: false };
  const value = Number(raw);
  return Number.isSafeInteger(value) && value <= 2_147_483_647
    ? { raw, value, valid: true }
    : { raw, value: null, valid: false };
}

export function numberDraft(value: number | null): string {
  return value === null ? "" : String(value);
}
