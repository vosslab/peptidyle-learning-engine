// j3_leave_return_fragment.ts - future report-safe public evidence for the J3 recovery journey.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
const MAX_ELAPSED_MS = 30 * 60 * 1000;

export const J3_VISIBLE_OUTCOME_CODES = [
  "visible_leave",
  "visible_return",
  "visible_start",
] as const;

export interface J3LeaveReturnFragment {
  readonly schemaVersion: 1;
  readonly journey: "J3";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId: string;
  readonly visibleOutcomeCodes: readonly (typeof J3_VISIBLE_OUTCOME_CODES)[number][];
  readonly diagnostics: readonly [];
}

/** Builds the narrow public J3 evidence record for later report integration. */
export function passedJ3LeaveReturnFragment(
  courseId: string,
  assignmentId: string,
  elapsedMs: number,
): J3LeaveReturnFragment {
  if (!UUID.test(courseId) || !UUID.test(assignmentId)) {
    throw new Error("J3 evidence requires public UUID identifiers");
  }
  if (!Number.isSafeInteger(elapsedMs) || elapsedMs < 0 || elapsedMs > MAX_ELAPSED_MS) {
    throw new Error("J3 evidence elapsed time is outside the allowed range");
  }
  return {
    schemaVersion: 1,
    journey: "J3",
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: J3_VISIBLE_OUTCOME_CODES,
    diagnostics: [],
  };
}
