// assessment_attempt_finish.ts - ordered backend document capture for Assessment Attempt finish.

export type BackendOwnedCapture = () => Promise<boolean>;

/** Whole-Attempt submission stays named apart from saving or changing one response. */
export function submitAttemptActionLabel(
  submissionState: "idle" | "submitting" | "submitted" | "error",
): string {
  return submissionState === "submitting" ? "Submitting Attempt..." : "Submit Attempt";
}

/** Leaves incomplete local input unsaved while preserving failures from a complete-response save. */
export async function saveCompleteResponseBeforeAttemptSubmission(
  responseIsComplete: boolean,
  save: () => Promise<boolean>,
  pendingSave?: Promise<boolean>,
): Promise<boolean> {
  if (pendingSave !== undefined && !(await pendingSave)) return false;
  return responseIsComplete ? save() : true;
}

/** Runs the active backend-document capture before the ordinary save/finalize lifecycle. */
export async function saveCapturedBackendOwnedResponse<T>(
  capture: BackendOwnedCapture | undefined,
  save: () => Promise<boolean>,
  finalize: () => Promise<T>,
): Promise<T | false> {
  if (capture !== undefined && !(await capture())) return false;
  if (!(await save())) return false;
  return finalize();
}
