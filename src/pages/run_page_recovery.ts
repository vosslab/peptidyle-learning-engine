// run_page_recovery.ts - page-local session restoration delivery helper.

export interface ReauthenticationAttemptMachine {
  readonly resumeAfterReauthentication: () => void;
  readonly submit: () => Promise<void>;
}

/**
 * A successful session check restores the existing buffered response and submits it through the
 * normal state-machine path. The machine owns the existing idempotency key, so this creates no
 * second logical submission.
 */
export async function resumeSessionAndRetry(
  getSession: () => Promise<unknown>,
  machine: ReauthenticationAttemptMachine,
): Promise<void> {
  await getSession();
  machine.resumeAfterReauthentication();
  await machine.submit();
}
