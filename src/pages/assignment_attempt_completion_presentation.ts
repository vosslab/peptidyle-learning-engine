// assignment_attempt_completion_presentation.ts - explicit Student copy for terminal Assignment Attempt states.

import type { AssignmentAttemptCompletion } from "../../generated/api/AssignmentAttemptCompletion";
import type { SubmissionAcknowledgement } from "../features/question_attempt/question_attempt_state";

export interface AssignmentAttemptCompletionPresentation {
  readonly eyebrow: string;
  readonly heading: string;
  readonly message: string;
}

/** Copy follows authoritative completion, not the absence of a successor attempt. */
export function assignmentAttemptCompletionPresentation(
  status: AssignmentAttemptCompletion,
  practiceAllowed: boolean | undefined,
): AssignmentAttemptCompletionPresentation {
  if (status === "inProgress") {
    return {
      eyebrow: "Assignment Attempt ended",
      heading: "Completion requirement not met",
      message:
        "Your response is recorded, but this Assignment Attempt did not meet the completion requirement.",
    };
  }
  return {
    eyebrow: "Assignment Attempt complete",
    heading:
      practiceAllowed === undefined
        ? "Assignment Attempt complete"
        : practiceAllowed
          ? "Keep practicing with fresh Question Seeds"
          : "This Assignment Attempt is complete",
    message: "Your completed Assignment Attempt is recorded.",
  };
}

/** The feedback action names the exact successor or terminal transition. */
export function submissionAdvanceLabel(
  acknowledgement: SubmissionAcknowledgement,
): string | undefined {
  if (acknowledgement.nextPending) return "Refresh for the next question";
  if (acknowledgement.nextIssued !== null) return undefined;
  return acknowledgement.assignmentAttemptCompletion === "completed"
    ? "View completed Assignment Attempt"
    : "View Assignment Attempt status";
}
