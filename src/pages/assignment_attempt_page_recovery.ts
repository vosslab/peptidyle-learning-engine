// assignment_attempt_page_recovery.ts - page-local session restoration delivery helper.

import type { SubmissionOutcome } from "../features/question_attempt/question_attempt_state";

export interface ReauthenticationAttemptMachine {
  readonly resumeAfterReauthentication: () => void;
  readonly submit: () => Promise<SubmissionOutcome>;
}

/**
 * A successful session check restores the existing buffered response and submits it through the
 * normal state-machine path. The machine owns the existing idempotency key, so this creates no
 * second logical submission.
 */
export async function resumeSessionAndRetry(
  getSession: () => Promise<unknown>,
  machine: ReauthenticationAttemptMachine,
): Promise<SubmissionOutcome> {
  await getSession();
  machine.resumeAfterReauthentication();
  return machine.submit();
}
