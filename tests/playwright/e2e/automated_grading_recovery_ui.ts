// Stable, learner-safe vocabulary shared by the automated-grading recovery journey.

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
const privateFieldNames = new Set([
  "answer",
  "correct",
  "correctness",
  "feedback",
  "grading",
  "gradingkey",
  "learnerresponse",
  "pointsearned",
  "pointspossible",
  "rawkey",
  "response",
  "result",
  "score",
  "solution",
  "submittedresponse",
]);

function isPrivateFieldName(key: string): boolean {
  const normalized = key.replace(/[^A-Za-z0-9]/gu, "").toLowerCase();
  if (privateFieldNames.has(normalized)) return true;
  const tokens = key
    .replace(/([a-z0-9])([A-Z])/gu, "$1 $2")
    .split(/[^A-Za-z0-9]+/u)
    .map((token) => token.toLowerCase());
  return tokens.some((token) => privateFieldNames.has(token));
}

/** Identifies the one Instructor mutation exercised by the recovery journey. */
export function isInstructorRetryPost(method: string, pathname: string): boolean {
  return method === "POST" && instructorRetryPath.test(pathname);
}

/** Identifies the Instructor metadata list without matching action subroutes. */
export function isInstructorOperationsListGet(method: string, pathname: string): boolean {
  return method === "GET" && instructorOperationsListPath.test(pathname);
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
      if (isPrivateFieldName(key)) return `${path}.${key} is a private answer field`;
      const violation = visit(item, `${path}.${key}`);
      if (violation !== null) return violation;
    }
    return null;
  }

  return visit(value, "response");
}

/**
 * Verifies response redaction on a completed learner variant after the caller
 * has passed it through the production browser decoder.
 */
export function completedLearnerReceiptViolation(value: unknown): string | null {
  if (value === null || typeof value !== "object") {
    return "response is not a decoded completed learner receipt";
  }
  const decoded = value as { kind?: unknown; attempt?: unknown };
  if (decoded.kind !== "completed") return "response is not a completed learner receipt";
  if (decoded.attempt === null || typeof decoded.attempt !== "object") {
    return "response.attempt is not a decoded learner attempt";
  }
  const attempt = decoded.attempt as { response?: unknown };
  if (attempt.response !== null) {
    return "response.attempt.response contains the submitted learner response";
  }
  return null;
}

export function isLearnerSubmissionPost(method: string, pathname: string): boolean {
  return method === "POST" && /\/submissions$/u.test(pathname);
}

export function isLearnerStatusGet(method: string, pathname: string): boolean {
  return method === "GET" && /\/submission-status$/u.test(pathname);
}
