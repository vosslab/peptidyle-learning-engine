// Pure learner-facing presentation state for one Coursework item.

import type { AssessmentAttemptCompletion } from "../../generated/api/AssessmentAttemptCompletion";
import type { AssessmentStartDecision } from "../../generated/api/AssessmentStartDecision";

export type StudentCourseworkDisplayState =
  "upcoming" | "available" | "inProgress" | "completed" | "missed";

export interface StudentCourseworkDisplay {
  readonly state: StudentCourseworkDisplayState;
  readonly stateLabel: string;
  readonly completionLabel: string;
  readonly actionVerb: "Resume" | "Review" | "Open";
}

const DUE_SOON_WINDOW_MILLIS = 7 * 24 * 60 * 60 * 1000;

/** Uses the API's server-evaluated instant and the existing rolling seven-day window. */
export function isInStudentDueSoonWindow(
  dueAtMillis: number | null,
  evaluatedAtMillis: number,
): boolean {
  return (
    dueAtMillis !== null &&
    dueAtMillis >= evaluatedAtMillis &&
    dueAtMillis < evaluatedAtMillis + DUE_SOON_WINDOW_MILLIS
  );
}

/** Completed view membership is a submitted Attempt fact, separate from score or latest state. */
export function hasSubmittedStudentAttempt(submittedAttemptCount: number): boolean {
  return Number.isInteger(submittedAttemptCount) && submittedAttemptCount > 0;
}

/** Maps server-owned access and Attempt evidence to a compact learner-facing state. */
export function studentCourseworkDisplay(
  startDecision: AssessmentStartDecision,
  completion: AssessmentAttemptCompletion | null,
  canResumeAssessmentAttempt: boolean,
): StudentCourseworkDisplay {
  // The overview offers authorized same-Attempt Resume issuance; otherwise it
  // exposes previous Attempts and any permitted start action without starting work.
  const actionVerb = canResumeAssessmentAttempt
    ? "Resume"
    : completion === "completed"
      ? "Review"
      : "Open";
  if (completion === "completed") {
    return {
      state: "completed",
      stateLabel: "Completed",
      completionLabel: "Completed",
      actionVerb,
    };
  }
  if (canResumeAssessmentAttempt) {
    return {
      state: "inProgress",
      stateLabel: "In progress",
      completionLabel: "In progress",
      actionVerb,
    };
  }
  if (startDecision === "not_yet_available") {
    return {
      state: "upcoming",
      stateLabel: "Upcoming",
      completionLabel: "Not started",
      actionVerb,
    };
  }
  if (startDecision === "may_start") {
    return {
      state: "available",
      stateLabel: "Available",
      completionLabel: "Not started",
      actionVerb,
    };
  }
  return { state: "missed", stateLabel: "Missed", completionLabel: "Not completed", actionVerb };
}
