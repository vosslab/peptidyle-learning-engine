// Compact, typed route references rendered for people instead of internal UUIDs.

import type { AssignmentPublicId } from "../../generated/api/AssignmentPublicId";
import type { CoursePublicId } from "../../generated/api/CoursePublicId";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { RunPublicId } from "../../generated/api/RunPublicId";
import type { WorkspacePublicId } from "../../generated/api/WorkspacePublicId";
import { normalizeQuestionIdSyntax } from "../question_id";

declare const routeReferenceBrand: unique symbol;

type BrandedRouteReference<Kind extends string> = string & {
  readonly [routeReferenceBrand]: Kind;
};

export type CourseRouteReference = BrandedRouteReference<"course">;
export type AssignmentRouteReference = BrandedRouteReference<"assignment">;
export type RunRouteReference = BrandedRouteReference<"run">;
export type WorkspaceRouteReference = BrandedRouteReference<"workspace">;
export type ProblemRouteReference = BrandedRouteReference<"question">;
export type PublicRouteReference =
  CourseRouteReference | AssignmentRouteReference | RunRouteReference | WorkspaceRouteReference;

const MAX_PUBLIC_ROUTE_NUMBER = 2_147_483_647;
const PUBLIC_ROUTE_PATTERN = /^(C|A|R|W)-([1-9][0-9]{0,9})$/u;

function publicNumber(value: number, label: string): string {
  if (!Number.isSafeInteger(value) || value < 1 || value > MAX_PUBLIC_ROUTE_NUMBER) {
    throw new Error(`${label} must be a positive 31-bit integer`);
  }
  return String(value);
}

export function courseRouteReference(value: CoursePublicId): CourseRouteReference {
  return `C-${publicNumber(value, "course public ID")}` as CourseRouteReference;
}

export function assignmentRouteReference(value: AssignmentPublicId): AssignmentRouteReference {
  return `A-${publicNumber(value, "assignment public ID")}` as AssignmentRouteReference;
}

export function runRouteReference(value: RunPublicId): RunRouteReference {
  return `R-${publicNumber(value, "run public ID")}` as RunRouteReference;
}

export function workspaceRouteReference(value: WorkspacePublicId): WorkspaceRouteReference {
  return `W-${publicNumber(value, "workspace public ID")}` as WorkspaceRouteReference;
}

export function problemRouteReference(questionId: QuestionId): ProblemRouteReference {
  const normalized = parseProblemRouteReference(questionId);
  if (normalized === null) throw new Error("question ID must use Crockford Base32");
  return normalized;
}

export function parsePublicRouteReference(value: string): PublicRouteReference | null {
  const match = PUBLIC_ROUTE_PATTERN.exec(value);
  if (match === null) return null;
  const numeric = Number(match[2]);
  if (!Number.isSafeInteger(numeric) || numeric > MAX_PUBLIC_ROUTE_NUMBER) return null;
  return value as PublicRouteReference;
}

export function parseProblemRouteReference(value: string): ProblemRouteReference | null {
  return normalizeQuestionIdSyntax(value) as ProblemRouteReference | null;
}
