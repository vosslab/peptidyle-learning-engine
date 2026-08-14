// instructor_gradebook_j5.ts - public-only evidence shape for the J5 browser journey.

const J5_CODES = ["visible_gradebook", "visible_run_history"] as const;
const J5_SCORE_CODES = [
  "visible_gradebook",
  "visible_score_summary",
  "visible_two_run_history",
] as const;

export type J5VisibleOutcomeCode = (typeof J5_CODES)[number];
export type J5SummaryVisibleOutcomeCode = (typeof J5_SCORE_CODES)[number];

/** Returns the exact-course selector for the instructor's rendered Gradebook link. */
export function instructorGradebookLinkSelector(courseReference: string): string {
  return `a[href="/instructor/courses/${courseReference}/gradebook"]`;
}

/**
 * This isolated fragment is intentionally not connected to the shared report
 * renderer until the M5 integration owner expands its closed journey contract.
 */
export interface J5JourneyFragment {
  readonly schemaVersion: 1;
  readonly journey: "J5";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseReference: string;
  readonly assignmentReference: string;
  readonly visibleOutcomeCodes: readonly J5VisibleOutcomeCode[];
  readonly diagnostics: readonly [];
}

export function passedW4Fragment(
  courseReference: string,
  assignmentReference: string,
  elapsedMs: number,
): J5JourneyFragment {
  return {
    schemaVersion: 1,
    journey: "J5",
    status: "PASS",
    elapsedMs,
    courseReference,
    assignmentReference,
    visibleOutcomeCodes: J5_CODES,
    diagnostics: [],
  };
}

/**
 * WP-S2's browser-only receipt. WP-E1 will connect this schema-v2-shaped
 * evidence to the protected state and redacted report; do not retrofit it
 * into the historical schema-v1 renderer.
 */
export interface J5SummaryEvidence {
  readonly schemaVersion: 2;
  readonly journey: "J5";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseReference: string;
  readonly assignmentReference: string;
  readonly visibleOutcomeCodes: readonly J5SummaryVisibleOutcomeCode[];
  readonly diagnostics: readonly [];
}

/** Builds the closed J5 evidence after the browser has visibly proved score and history. */
export function passedJ5SummaryEvidence(
  courseReference: string,
  assignmentReference: string,
  elapsedMs: number,
): J5SummaryEvidence {
  return {
    schemaVersion: 2,
    journey: "J5",
    status: "PASS",
    elapsedMs,
    courseReference,
    assignmentReference,
    visibleOutcomeCodes: J5_SCORE_CODES,
    diagnostics: [],
  };
}
