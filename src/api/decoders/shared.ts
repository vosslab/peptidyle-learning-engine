// Shared strict primitives for browser-visible API DTOs.

import { MAX_QUESTION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_QUESTION_TITLE_UNICODE_SCALARS";
import type { BackendCapabilities } from "../../../generated/api/BackendCapabilities";
import type { Capability } from "../../../generated/api/Capability";
import type { CatalogLifecycle } from "../../../generated/api/CatalogLifecycle";
import type { License } from "../../../generated/api/License";
import type { ProblemVersionRef } from "../../../generated/api/ProblemVersionRef";
import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionMetadata } from "../../../generated/api/QuestionMetadata";
import type { TaxonomyTerm } from "../../../generated/api/TaxonomyTerm";
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
  "perQuestionTiming",
  "printExport",
  "offlinePreview",
] as const satisfies ReadonlyArray<Capability>;
export const MAX_CURSOR_LENGTH = 512;
/** Matches the server-owned PageSize::MAX for every cursor-list response. */
export const MAX_CURSOR_PAGE_ITEMS = 100;
/** Largest public route number accepted by the Rust public-reference contract. */
export const MAX_PUBLIC_ROUTE_NUMBER = 2_147_483_647;
export const QUESTION_BACKENDS = [
  "native",
  "webwork",
  "qti",
  "h5p",
  "imathas",
] as const satisfies ReadonlyArray<QuestionBackend>;

export const MAX_CATALOG_PAGE_ITEMS = MAX_CURSOR_PAGE_ITEMS;
export const MAX_CATALOG_CAPABILITY_FACETS = CAPABILITIES.length;
export const MAX_CATALOG_LICENSE_FACETS = 6;
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

export function decodeBackendCapabilities(value: unknown, path: string): BackendCapabilities {
  return decodeArray(value, path, decodeCapability);
}

export function decodeProblemVersionRef(
  value: unknown,
  path: string,
  strict = false,
): ProblemVersionRef {
  const record = decodeRecord(value, path);
  if (strict) requireOnlyFields(record, path, ["problem", "version"]);
  const decoded = {
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
  } satisfies ProblemVersionRef;
  return decoded;
}

export function decodeTaxonomyTerm(value: unknown, path: string, strict = false): TaxonomyTerm {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["scheme", "code", "label"]);
  }
  const decoded = {
    scheme: decodeNonemptyString(field(record, "scheme", path), `${path}.scheme`),
    code: decodeNonemptyString(field(record, "code", path), `${path}.code`),
    label: decodeNonemptyString(field(record, "label", path), `${path}.label`),
  } satisfies TaxonomyTerm;
  return decoded;
}

export function decodeLicense(value: unknown, path: string, strict = false): License {
  const record = decodeRecord(value, path);
  const tag = kind(record, path);
  switch (tag) {
    case "allRightsReserved":
    case "ccBy":
    case "ccBySa":
    case "ccByNc":
    case "cc0":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: tag };
    case "other": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "spdx"]);
      }
      const decoded = {
        kind: tag,
        spdx: decodeNonemptyString(field(record, "spdx", path), `${path}.spdx`),
      } satisfies License;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known license kind");
  }
}

export function decodeQuestionMetadata(
  value: unknown,
  path: string,
  strict = false,
): QuestionMetadata {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["title", "tags", "taxonomy", "license", "language"]);
  }
  const decoded = {
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    tags: decodeArray(field(record, "tags", path), `${path}.tags`, decodeString),
    taxonomy: decodeArray(field(record, "taxonomy", path), `${path}.taxonomy`, (term, termPath) =>
      decodeTaxonomyTerm(term, termPath, strict),
    ),
    license: decodeLicense(field(record, "license", path), `${path}.license`, strict),
    language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
  } satisfies QuestionMetadata;
  return decoded;
}

export function decodeCatalogLifecycle(
  value: unknown,
  path: string,
  strict = false,
): CatalogLifecycle {
  const record = decodeRecord(value, path);
  const lifecycle = state(record, path);
  switch (lifecycle) {
    case "published":
      if (strict) {
        requireOnlyFields(record, path, ["state"]);
      }
      return { state: lifecycle };
    case "deprecated":
    case "archived": {
      if (strict) {
        requireOnlyFields(record, path, ["state", "reason"]);
      }
      const decoded = {
        state: lifecycle,
        reason: decodeNonemptyString(field(record, "reason", path), `${path}.reason`),
      } satisfies CatalogLifecycle;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.state`, "a known catalog lifecycle");
  }
}
