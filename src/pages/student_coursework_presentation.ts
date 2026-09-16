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

/** Maps server-owned access and Attempt evidence to a compact learner-facing state. */
export function studentCourseworkDisplay(
  startDecision: AssessmentStartDecision,
  completion: AssessmentAttemptCompletion | null,
  canResumeAssessmentAttempt: boolean,
): StudentCourseworkDisplay {
  // The overview redirects an authorized active Attempt; otherwise it exposes
  // previous Attempts and any permitted start action without starting work.
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
