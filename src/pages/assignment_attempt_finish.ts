// assignment_attempt_finish.ts - ordered backend document capture for Assignment Attempt finish.

export type BackendOwnedCapture = () => Promise<boolean>;

/** Runs the active backend-document capture before the ordinary save/finalize lifecycle. */
export async function saveCapturedBackendOwnedResponse(
  capture: BackendOwnedCapture | undefined,
  save: () => Promise<boolean>,
  finalize: () => Promise<unknown>,
): Promise<boolean> {
  if (capture !== undefined && !(await capture())) return false;
  if (!(await save())) return false;
  await finalize();
  return true;
}
