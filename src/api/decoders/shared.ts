// Shared strict primitives for browser-visible API DTOs.

import { MAX_QUESTION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_QUESTION_TITLE_UNICODE_SCALARS";
import type { QuestionBackendCapabilities } from "../../../generated/api/QuestionBackendCapabilities";
import type { Capability } from "../../../generated/api/Capability";
import type { QuestionRevisionAvailability } from "../../../generated/api/QuestionRevisionAvailability";
import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionCitation } from "../../../generated/api/QuestionCitation";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionMetadata } from "../../../generated/api/QuestionMetadata";
import type { QuestionClassification } from "../../../generated/api/QuestionClassification";
import type { CursorPage } from "../contracts";
import {
  DecodeError,
  decodeArray,
  decodeField,
  decodeNonemptyString,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
  decodeUuid,
  type Decoder,
} from "../decoder";

const CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "questionAttemptTimeLimit",
  "printExport",
  "offlinePreview",
] as const satisfies ReadonlyArray<Capability>;
export const MAX_CURSOR_LENGTH = 512;
/** Matches the server-owned PageSize::MAX for every cursor-list response. */
export const MAX_CURSOR_PAGE_ITEMS = 100;
/** Largest public route number accepted by the Rust public-reference contract. */
export const MAX_PUBLIC_ROUTE_NUMBER = 2_147_483_647;
export const QUESTION_BACKENDS = [
  "ple",
  "webwork",
  "qti",
  "h5p",
  "imathas",
] as const satisfies ReadonlyArray<QuestionBackend>;

export const MAX_QUESTION_SEARCH_PAGE_ITEMS = MAX_CURSOR_PAGE_ITEMS;
export const MAX_QUESTION_SEARCH_CAPABILITY_FACETS = CAPABILITIES.length;
export const MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS = 3;
export const MINIMUM_STATISTICS_COHORT_SIZE = 5;
export const STATISTICS_DURATION_ESTIMATES_SECONDS = [
  1, 5, 15, 30, 60, 120, 300, 900, 3_600, 86_400,
] as const;
export const MAX_PUBLICATION_SEMANTIC_ENTRIES = 100;
const MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS = 200;

export function decodeEnvelopeTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (title.trim().length === 0) {
    throw new DecodeError(path, "a title containing non-whitespace content");
  }
  if (Array.from(title).length > MAX_QUESTION_TITLE_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `a title no longer than ${MAX_QUESTION_TITLE_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return title;
}

export function decodeAssignmentTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (title.trim().length === 0) {
    throw new DecodeError(path, "an assignment title containing non-whitespace content");
  }
  if (Array.from(title).length > MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `an assignment title no longer than ${MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return title;
}

export function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
}

export function requireOnlyFields(
  record: Record<string, unknown>,
  path: string,
  allowed: ReadonlyArray<string>,
): void {
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by this response contract");
    }
  }
}

export function decodeBoundedArray<T>(
  value: unknown,
  path: string,
  maximum: number,
  decodeElement: Decoder<T>,
): Array<T> {
  const decoded = decodeArray(value, path, decodeElement);
  if (decoded.length > maximum) {
    throw new DecodeError(path, `an array with at most ${maximum} entries`);
  }
  return decoded;
}

/** Decodes one opaque cursor returned by a bounded list response. */
export function decodeCursor(value: unknown, path: string): string {
  const cursor = decodeNonemptyString(value, path);
  if (cursor.length > MAX_CURSOR_LENGTH) {
    throw new DecodeError(path, `a cursor no longer than ${MAX_CURSOR_LENGTH} characters`);
  }
  return cursor;
}

/**
 * Decodes the exact common cursor-page envelope used by browser list routes.
 *
 * The server's shared pagination contract caps every page at 100 rows. Keeping
 * the same cap at this untrusted browser boundary prevents a malformed response
 * from bypassing the bounded transport contract before individual items decode.
 */
export function decodeCursorPage<T>(
  value: unknown,
  path: string,
  decodeItem: Decoder<T>,
): CursorPage<T> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_CURSOR_PAGE_ITEMS,
      decodeItem,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}

export function kind(record: Record<string, unknown>, path: string): string {
  return decodeString(field(record, "kind", path), `${path}.kind`);
}

export function state(record: Record<string, unknown>, path: string): string {
  return decodeString(field(record, "state", path), `${path}.state`);
}

export function decodeTimestamp(value: unknown, path: string): number {
  return decodeSafeInteger(value, path);
}

export function decodeIdentifier(value: unknown, path: string): string {
  return decodeUuid(value, path);
}

/** Decodes the canonical, browser-visible identity of an immutable question. */
export function decodeQuestionId(value: unknown, path: string): QuestionId {
  const questionId = decodeString(value, path);
  if (!/^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u.test(questionId)) {
    throw new DecodeError(path, "a canonical Question ID");
  }
  return questionId;
}

/** Decodes the positive version number within one published Question lineage. */
export function decodePositiveQuestionRevisionNumber(value: unknown, path: string): number {
  const revisionNumber = decodeSafeInteger(value, path);
  if (revisionNumber < 1) {
    throw new DecodeError(path, "a positive Question Revision Number");
  }
  return revisionNumber;
}

/** Decodes a compact positive database identity that is safe to show to people. */
export function decodePublicRouteNumber(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 1 || decoded > MAX_PUBLIC_ROUTE_NUMBER) {
    throw new DecodeError(path, `an integer from 1 through ${MAX_PUBLIC_ROUTE_NUMBER}`);
  }
  return decoded;
}

export function decodeSha256(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[0-9a-f]{64}$/i.test(decoded)) {
    throw new DecodeError(path, "a 64-character SHA-256 hexadecimal digest");
  }
  return decoded;
}

export function decodeCapability(value: unknown, path: string): Capability {
  return decodeStringEnum(value, path, CAPABILITIES);
}

export function decodeQuestionBackendCapabilities(
  value: unknown,
  path: string,
): QuestionBackendCapabilities {
  return decodeArray(value, path, decodeCapability);
}

export function decodeQuestionRevisionReference(
  value: unknown,
  path: string,
  strict = false,
): QuestionRevisionReference {
  const record = decodeRecord(value, path);
  if (strict) requireOnlyFields(record, path, ["questionId", "revisionNumber"]);
  const decoded = {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    revisionNumber: decodePositiveQuestionRevisionNumber(
      field(record, "revisionNumber", path),
      `${path}.revisionNumber`,
    ),
  } satisfies QuestionRevisionReference;
  return decoded;
}

export function decodeQuestionClassification(
  value: unknown,
  path: string,
  strict = false,
): QuestionClassification {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["system", "code", "name"]);
  }
  const decoded = {
    system: decodeNonemptyString(field(record, "system", path), `${path}.system`),
    code: decodeNonemptyString(field(record, "code", path), `${path}.code`),
    name: decodeNonemptyString(field(record, "name", path), `${path}.name`),
  } satisfies QuestionClassification;
  return decoded;
}

export function decodeQuestionLicense(value: unknown, path: string): QuestionLicense {
  return decodeStringEnum<QuestionLicense>(value, path, ["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"]);
}

export function decodeQuestionCitation(value: unknown, path: string): QuestionCitation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["citationUrl", "citationText"]);
  const citationUrl = decodeNullable(
    field(record, "citationUrl", path),
    `${path}.citationUrl`,
    decodeNonemptyString,
  );
  const citationText = decodeNullable(
    field(record, "citationText", path),
    `${path}.citationText`,
    decodeNonemptyString,
  );
  if (citationUrl === null && citationText === null) {
    throw new DecodeError(path, "a Question Citation with Citation URL, Citation Text, or both");
  }
  return { citationUrl, citationText };
}

export function decodeQuestionMetadata(
  value: unknown,
  path: string,
  strict = false,
): QuestionMetadata {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "title",
      "questionDescription",
      "tags",
      "classifications",
      "questionLicense",
      "questionCitation",
      "language",
    ]);
  }
  const decoded = {
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    questionDescription: decodeNonemptyString(
      field(record, "questionDescription", path),
      `${path}.questionDescription`,
    ),
    tags: decodeArray(field(record, "tags", path), `${path}.tags`, decodeString),
    classifications: decodeArray(
      field(record, "classifications", path),
      `${path}.classifications`,
      (classification, classificationPath) =>
        decodeQuestionClassification(classification, classificationPath, strict),
    ),
    questionLicense: decodeNullable(
      field(record, "questionLicense", path),
      `${path}.questionLicense`,
      decodeQuestionLicense,
    ),
    questionCitation: decodeNullable(
      field(record, "questionCitation", path),
      `${path}.questionCitation`,
      decodeQuestionCitation,
    ),
    language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
  } satisfies QuestionMetadata;
  return decoded;
}

export function decodeQuestionRevisionAvailability(
  value: unknown,
  path: string,
  strict = false,
): QuestionRevisionAvailability {
  const record = decodeRecord(value, path);
  const availability = decodeStringEnum(
    field(record, "availability", path),
    `${path}.availability`,
    ["available", "archived"],
  );
  switch (availability) {
    case "available":
      if (strict) {
        requireOnlyFields(record, path, ["availability"]);
      }
      return { availability };
    case "archived": {
      if (strict) {
        requireOnlyFields(record, path, ["availability", "reason"]);
      }
      const decoded = {
        availability,
        reason: decodeNonemptyString(field(record, "reason", path), `${path}.reason`),
      } satisfies QuestionRevisionAvailability;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.availability`, "a known Question Revision Availability");
  }
}
