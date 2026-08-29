// Stable, Student-safe vocabulary shared by the automated-grading recovery journey.

export const AUTOMATED_GRADING_RECOVERY_LABELS = {
  responseReceived: "Response received",
  checkGradingStatus: "Check grading status",
  gradingOperations: "Grading operations",
  gradebook: "Gradebook",
} as const;

export const automatedGradingRetryName = /^Retry automated grading for /u;

const instructorRetryPath = /\/grading-operations\/GO-[1-9][0-9]{0,9}\/retry$/u;
const instructorOperationsListPath =
  /\/api\/courses\/[^/]+\/assignments\/[^/]+\/grading-operations$/u;
/** Identifies the one Instructor mutation exercised by the recovery journey. */
export function isInstructorRetryPost(method: string, pathname: string): boolean {
  return method === "POST" && instructorRetryPath.test(pathname);
}

/** Identifies the Instructor metadata list without matching action subroutes. */
export function isInstructorOperationsListGet(method: string, pathname: string): boolean {
  return method === "GET" && instructorOperationsListPath.test(pathname);
}

export function isStudentSubmissionPost(method: string, pathname: string): boolean {
  return method === "POST" && /\/submissions$/u.test(pathname);
}

export function isStudentStatusGet(method: string, pathname: string): boolean {
  return method === "GET" && /\/submission-status$/u.test(pathname);
}
