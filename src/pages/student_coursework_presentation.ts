// Pure learner-facing presentation state for one Coursework item.

import type { AssessmentAttemptCompletion } from "../../generated/api/AssessmentAttemptCompletion";
import type { AssessmentStartDecision } from "../../generated/api/AssessmentStartDecision";

export type StudentCourseworkDisplayState =
  "upcoming" | "available" | "inProgress" | "completed" | "missed";

export interface StudentCourseworkDisplay {
  readonly state: StudentCourseworkDisplayState;
  readonly stateLabel: string;
  readonly completionLabel: string;
}

/** Maps server-owned access and Attempt evidence to a compact learner-facing state. */
export function studentCourseworkDisplay(
  startDecision: AssessmentStartDecision,
  completion: AssessmentAttemptCompletion | null,
  canResumeAssessmentAttempt: boolean,
): StudentCourseworkDisplay {
  if (completion === "completed") {
    return { state: "completed", stateLabel: "Completed", completionLabel: "Completed" };
  }
  if (canResumeAssessmentAttempt) {
    return { state: "inProgress", stateLabel: "In progress", completionLabel: "In progress" };
  }
  if (startDecision === "not_yet_available") {
    return { state: "upcoming", stateLabel: "Upcoming", completionLabel: "Not started" };
  }
  if (startDecision === "may_start") {
    return { state: "available", stateLabel: "Available", completionLabel: "Not started" };
  }
  return { state: "missed", stateLabel: "Missed", completionLabel: "Not completed" };
}
