// timer.ts - server-free MOD-TIME fallback for the mock browser runtime.

import type { TimerEvaluation, TimerVerdict } from "../../wasm/index";

function requireSafeTimestamp(value: number, label: string): void {
  if (!Number.isSafeInteger(value)) {
    throw new Error(`${label} must be a safe integer Unix-millisecond timestamp`);
  }
}

/**
 * Mirrors the Rust timer contract only for the server-free reference UI.
 * A deployed client replaces this with the authoritative API fallback.
 */
export function timerVerdictInMock(evaluation: TimerEvaluation): Promise<TimerVerdict> {
  const { policy, timer, evaluatedAt, pauseExtensionMillis } = evaluation;
  requireSafeTimestamp(timer.issuedAt, "issuedAt");
  requireSafeTimestamp(evaluatedAt, "evaluatedAt");
  requireSafeTimestamp(pauseExtensionMillis, "pauseExtensionMillis");
  if (pauseExtensionMillis < 0) {
    throw new Error("pauseExtensionMillis cannot be negative");
  }
  if (evaluatedAt < timer.issuedAt) {
    throw new Error("evaluatedAt cannot predate issuedAt");
  }
  if (timer.submittedAt !== null) {
    requireSafeTimestamp(timer.submittedAt, "submittedAt");
    if (timer.submittedAt < timer.issuedAt) {
      throw new Error("submittedAt cannot predate issuedAt");
    }
    if (timer.submittedAt > evaluatedAt) {
      throw new Error("submittedAt cannot follow evaluatedAt");
    }
  }

  if (policy.kind === "untimed") {
    if (timer.deadline !== null || pauseExtensionMillis !== 0) {
      throw new Error("untimed evaluations cannot carry a deadline or pause extension");
    }
    return Promise.resolve("untimed");
  }
  if (timer.deadline === null) {
    throw new Error("timed evaluations require a server deadline");
  }
  requireSafeTimestamp(timer.deadline, "deadline");
  if (timer.deadline < timer.issuedAt) {
    throw new Error("deadline cannot predate issuedAt");
  }

  const effectiveDeadline = timer.deadline + pauseExtensionMillis;
  const graceDeadline = effectiveDeadline + policy.graceSeconds * 1_000;
  requireSafeTimestamp(effectiveDeadline, "effective deadline");
  requireSafeTimestamp(graceDeadline, "grace deadline");
  const observedAt = timer.submittedAt ?? evaluatedAt;

  if (observedAt <= effectiveDeadline) {
    return Promise.resolve(timer.submittedAt === null ? "open" : "submittedOnTime");
  }
  if (observedAt <= graceDeadline) {
    return Promise.resolve(timer.submittedAt === null ? "gracePeriod" : "submittedWithinGrace");
  }
  return Promise.resolve("timedOut");
}
