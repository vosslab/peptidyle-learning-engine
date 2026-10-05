// Strict browser wire contracts for the normal passwordless email-code ceremony.

import { decodeField, decodeRecord, decodeTrue, decodeUuid } from "./decoder";
import { requireOnlyFields } from "./decoders/shared";

export interface StartedEmailCodeSignIn {
  readonly emailCodeRequested: true;
  /** Opaque browser-visible ceremony routing ID, never an account identifier. */
  readonly challengeId: string;
}

export interface CompletedEmailCodeSignIn {
  readonly authenticated: true;
}

/** Normal Student and Instructor passwordless sign-in, outside the Live Demo seam. */
export interface EmailCodeClient {
  readonly startEmailCodeSignIn: (email: string) => Promise<StartedEmailCodeSignIn>;
  readonly completeEmailCodeSignIn: (
    challengeId: string,
    code: string,
  ) => Promise<CompletedEmailCodeSignIn>;
}

function closedRecord(
  value: unknown,
  path: string,
  fields: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, fields);
  for (const field of fields) decodeField(record, field, path);
  return record;
}

export function decodeStartedEmailCodeSignIn(
  value: unknown,
  path = "response",
): StartedEmailCodeSignIn {
  const record = closedRecord(value, path, ["emailCodeRequested", "challengeId"]);
  return {
    emailCodeRequested: decodeTrue(record.emailCodeRequested, `${path}.emailCodeRequested`),
    challengeId: decodeUuid(record.challengeId, `${path}.challengeId`),
  };
}

export function decodeCompletedEmailCodeSignIn(
  value: unknown,
  path = "response",
): CompletedEmailCodeSignIn {
  const record = closedRecord(value, path, ["authenticated"]);
  return { authenticated: decodeTrue(record.authenticated, `${path}.authenticated`) };
}

/** Validates the opaque server-issued route segment before a browser dispatches it. */
export function decodeEmailCodeChallengeId(value: unknown, path = "challengeId"): string {
  return decodeUuid(value, path);
}
