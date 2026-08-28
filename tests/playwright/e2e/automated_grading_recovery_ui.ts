// Stable, learner-safe vocabulary shared by the automated-grading recovery journey.

export const AUTOMATED_GRADING_RECOVERY_LABELS = {
  responseReceived: "Response received",
  checkGradingStatus: "Check grading status",
  gradingOperations: "Grading operations",
  gradebook: "Gradebook",
} as const;

export const automatedGradingRetryName = /^Retry grading operation GO-[1-9][0-9]*$/u;

const instructorRetryPath = /\/grading-operations\/GO-[1-9][0-9]{0,9}\/retry$/u;
const privateFieldName =
  /(?:answer|correct|feedback|gradingKey|learnerResponse|pointsEarned|pointsPossible|rawKey|response|result|score|solution|submittedResponse)/iu;

/** Identifies the one Instructor mutation exercised by the recovery journey. */
export function isInstructorRetryPost(method: string, pathname: string): boolean {
  return method === "POST" && instructorRetryPath.test(pathname);
}

/**
 * Reports the first answer-bearing field or private answer value in a JSON
 * projection. This is intentionally semantic: it checks private field
 * families and caller-supplied private values without coupling the journey to
 * the complete response shape or object ordering.
 */
export function answerFreeViolation(
  value: unknown,
  privateValues: ReadonlyArray<string> = [],
): string | null {
  function visit(candidate: unknown, path: string): string | null {
    if (typeof candidate === "string") {
      return privateValues.some(
        (privateValue) => privateValue.length > 0 && candidate.includes(privateValue),
      )
        ? `${path} contains a private answer value`
        : null;
    }
    if (Array.isArray(candidate)) {
      for (const [index, item] of candidate.entries()) {
        const violation = visit(item, `${path}[${index}]`);
        if (violation !== null) return violation;
      }
      return null;
    }
    if (candidate === null || typeof candidate !== "object") return null;
    for (const [key, item] of Object.entries(candidate)) {
      if (privateFieldName.test(key)) return `${path}.${key} is a private answer field`;
      const violation = visit(item, `${path}.${key}`);
      if (violation !== null) return violation;
    }
    return null;
  }

  return visit(value, "response");
}

export function isLearnerSubmissionPost(method: string, pathname: string): boolean {
  return method === "POST" && /\/submissions$/u.test(pathname);
}

export function isLearnerStatusGet(method: string, pathname: string): boolean {
  return method === "GET" && /\/submission-status$/u.test(pathname);
}
