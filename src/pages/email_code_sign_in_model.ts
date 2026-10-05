// Small UI policy for the browser-facing email-code ceremony.

import { ApiRequestError } from "../api/http_client/error";

export const EMAIL_CODE_LENGTH = 43;

export type EmailCodeStartFailure = "invalidEmail" | "unavailable" | "error";
export type EmailCodeCompletionFailure = "invalidCode" | "unavailable" | "error";

/** Keeps unavailable mail delivery distinct from a successful generic request. */
export function emailCodeStartFailure(error: unknown): EmailCodeStartFailure {
  if (!(error instanceof ApiRequestError)) return "error";
  if (error.status === 404 || error.status === 503) return "unavailable";
  return error.status === 422 ? "invalidEmail" : "error";
}

/** The service owns code checks; this only selects safe, direct recovery copy. */
export function emailCodeCompletionFailure(error: unknown): EmailCodeCompletionFailure {
  if (!(error instanceof ApiRequestError)) return "error";
  if (error.status === 404 || error.status === 503) return "unavailable";
  return error.status === 401 ? "invalidCode" : "error";
}
