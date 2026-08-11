// student_leave_resume_evidence.ts - report-safe evidence for visible leave and resume behavior.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
const MAX_ELAPSED_MS = 30 * 60 * 1000;

export const STUDENT_LEAVE_RESUME_VISIBLE_OUTCOME_CODES = [
  "visible_leave",
  "visible_return",
  "visible_start",
] as const;

export interface StudentLeaveResumeEvidence {
  readonly schemaVersion: 1;
  readonly journey: "J3";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId: string;
  readonly visibleOutcomeCodes: readonly (typeof STUDENT_LEAVE_RESUME_VISIBLE_OUTCOME_CODES)[number][];
  readonly diagnostics: readonly [];
}

/** Builds the narrow public evidence record for visible leave-and-resume behavior. */
export function passedStudentLeaveResumeEvidence(
  courseId: string,
  assignmentId: string,
  elapsedMs: number,
): StudentLeaveResumeEvidence {
  if (!UUID.test(courseId) || !UUID.test(assignmentId)) {
    throw new Error("leave-and-resume evidence requires public UUID identifiers");
  }
  if (!Number.isSafeInteger(elapsedMs) || elapsedMs < 0 || elapsedMs > MAX_ELAPSED_MS) {
    throw new Error("leave-and-resume evidence elapsed time is outside the allowed range");
  }
  return {
    schemaVersion: 1,
    journey: "J3",
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: STUDENT_LEAVE_RESUME_VISIBLE_OUTCOME_CODES,
    diagnostics: [],
  };
}
