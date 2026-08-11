// j4_policy_contrast_fragment.ts - narrow public evidence for the J4 UI contrast.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
const MAX_ELAPSED_MS = 30 * 60 * 1000;

export const J4_VISIBLE_OUTCOME_CODES = [
  "visible_back_action",
  "visible_exam_closed",
  "visible_exam_completion",
  "visible_mastery_completion",
  "visible_mastery_fresh_practice",
] as const;

export interface J4PolicyContrastFragment {
  readonly schemaVersion: 1;
  readonly journey: "J4";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly masteryAssignmentId: string;
  readonly examAssignmentId: string;
  readonly visibleOutcomeCodes: readonly (typeof J4_VISIBLE_OUTCOME_CODES)[number][];
  readonly diagnostics: readonly [];
}

/** Builds the later-report-safe record for a paired visible policy contrast. */
export function passedJ4PolicyContrastFragment(
  courseId: string,
  masteryAssignmentId: string,
  examAssignmentId: string,
  elapsedMs: number,
): J4PolicyContrastFragment {
  if (!UUID.test(courseId) || !UUID.test(masteryAssignmentId) || !UUID.test(examAssignmentId)) {
    throw new Error("J4 evidence requires public UUID identifiers");
  }
  if (!Number.isSafeInteger(elapsedMs) || elapsedMs < 0 || elapsedMs > MAX_ELAPSED_MS) {
    throw new Error("J4 evidence elapsed time is outside the allowed range");
  }
  return {
    schemaVersion: 1,
    journey: "J4",
    status: "PASS",
    elapsedMs,
    courseId,
    masteryAssignmentId,
    examAssignmentId,
    visibleOutcomeCodes: J4_VISIBLE_OUTCOME_CODES,
    diagnostics: [],
  };
}
