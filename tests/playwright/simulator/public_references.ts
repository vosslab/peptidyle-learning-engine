// public_references.ts - browser-visible course and assignment route references.

import { parsePublicRouteReference } from "../../../src/navigation/public_route";

function hasReferenceKind(value: unknown, prefix: "C-" | "A-"): value is string {
  if (typeof value !== "string" || !value.startsWith(prefix)) return false;
  return parsePublicRouteReference(value) !== null;
}

/** Validates the human-facing route reference copied from a course browser URL. */
export function isCourseReference(value: unknown): value is string {
  return hasReferenceKind(value, "C-");
}

/** Validates the human-facing route reference copied from an assignment browser URL. */
export function isAssignmentReference(value: unknown): value is string {
  return hasReferenceKind(value, "A-");
}
