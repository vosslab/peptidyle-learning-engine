// public_ids.ts - human-facing Course Instance and Assessment public IDs.

import { parsePublicRouteId } from "../../src/navigation/public_route";

function hasCanonicalPublicIdPrefix(value: unknown, prefix: "CI" | "A"): value is string {
  if (typeof value !== "string" || !value.startsWith(prefix)) return false;
  return parsePublicRouteId(value) !== null;
}

/** Validates the human-facing Course Instance ID copied from a route. */
export function isCourseInstanceId(value: unknown): value is string {
  return hasCanonicalPublicIdPrefix(value, "CI");
}

/** Validates the human-facing Assessment ID copied from a route. */
export function isAssessmentId(value: unknown): value is string {
  return hasCanonicalPublicIdPrefix(value, "A");
}
