// student_leave_resume_evidence.ts - report-safe evidence for visible leave and resume behavior.

import { isAssignmentReference, isCourseReference } from "./public_references";
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
  readonly courseReference: string;
  readonly assignmentReference: string;
  readonly visibleOutcomeCodes: readonly (typeof STUDENT_LEAVE_RESUME_VISIBLE_OUTCOME_CODES)[number][];
  readonly diagnostics: readonly [];
}

/** Builds the narrow public evidence record for visible leave-and-resume behavior. */
export function passedStudentLeaveResumeEvidence(
  courseReference: string,
  assignmentReference: string,
  elapsedMs: number,
): StudentLeaveResumeEvidence {
  if (!isCourseReference(courseReference) || !isAssignmentReference(assignmentReference)) {
    throw new Error("leave-and-resume evidence requires public route references");
  }
  if (!Number.isSafeInteger(elapsedMs) || elapsedMs < 0 || elapsedMs > MAX_ELAPSED_MS) {
    throw new Error("leave-and-resume evidence elapsed time is outside the allowed range");
  }
  return {
    schemaVersion: 1,
    journey: "J3",
    status: "PASS",
    elapsedMs,
    courseReference,
    assignmentReference,
    visibleOutcomeCodes: STUDENT_LEAVE_RESUME_VISIBLE_OUTCOME_CODES,
    diagnostics: [],
  };
}
