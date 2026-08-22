// run_completion_presentation.ts - explicit learner copy for terminal run states.

import type { RunCompletionStatus } from "../../generated/api/RunCompletionStatus";
import type { SubmissionAcknowledgement } from "../features/attempt/attempt_state";

export interface RunCompletionPresentation {
  readonly eyebrow: string;
  readonly heading: string;
  readonly message: string;
}

/** Copy follows authoritative completion, not the absence of a successor attempt. */
export function runCompletionPresentation(
  status: RunCompletionStatus,
  practiceAllowed: boolean | undefined,
): RunCompletionPresentation {
  if (status === "inProgress") {
    return {
      eyebrow: "Run ended",
      heading: "Completion requirement not met",
      message: "Your response is recorded, but this run did not meet the completion requirement.",
    };
  }
  return {
    eyebrow: "Run complete",
    heading:
      practiceAllowed === undefined
        ? "Run complete"
        : practiceAllowed
          ? "Keep practicing with a fresh variation"
          : "This run is complete",
    message: "Your completed run is recorded.",
  };
}

/** The feedback action names the exact successor or terminal transition. */
export function submissionAdvanceLabel(
  acknowledgement: SubmissionAcknowledgement,
): string | undefined {
  if (acknowledgement.nextPending) return "Refresh for the next question";
  if (acknowledgement.nextIssued !== null) return undefined;
  return acknowledgement.runCompletionStatus === "completed"
    ? "View completed run"
    : "View run status";
}
