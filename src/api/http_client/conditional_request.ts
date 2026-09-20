// HTTP If-Match / ETag adapter. Domain callers pass Edit or Revision Numbers.

import { ApiProtocolError } from "./error";

const MAX_POSITIVE_NUMBER = 9_223_372_036_854_775_807n;

function requirePositiveNumber(value: string, path: string, label: string): string {
  if (!/^[1-9][0-9]*$/u.test(value) || BigInt(value) > MAX_POSITIVE_NUMBER) {
    throw new ApiProtocolError(`API ${path} must be a positive ${label}`);
  }
  return value;
}

/** Encode one domain number as a strong HTTP If-Match validator. */
export function ifMatchHeaderForPositiveNumber(value: string, path: string, label: string): string {
  return `"${requirePositiveNumber(value, `${path} If-Match`, label)}"`;
}

/** Read the HTTP ETag as an unquoted positive domain number. */
export function numberFromQuotedPositiveHeader(
  response: Response,
  path: string,
  label: string,
): string {
  const etag = response.headers.get("etag");
  if (etag === null || !/^"[1-9][0-9]*"$/u.test(etag)) {
    throw new ApiProtocolError(`API response ${path} must include one strong numeric ${label}`);
  }
  const number = etag.slice(1, -1);
  return requirePositiveNumber(number, path, label);
}

/** Confirm the HTTP ETag encodes exactly this domain number. */
export function assertResponseMatchesPositiveNumber(
  response: Response,
  expected: string,
  path: string,
  label: string,
): string {
  const actual = numberFromQuotedPositiveHeader(response, path, label);
  if (actual !== requirePositiveNumber(expected, path, label)) {
    throw new ApiProtocolError(`API response ${path} quoted number must match its ${label}`);
  }
  return actual;
}
