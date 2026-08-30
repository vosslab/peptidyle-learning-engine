// Strict browser decoding for personal and institution problem-curation APIs.

import { MAX_CATALOG_BYLINE_FILTERS } from "../../../generated/api/MAX_CATALOG_BYLINE_FILTERS";
import { MAX_CATALOG_TAG_FILTERS } from "../../../generated/api/MAX_CATALOG_TAG_FILTERS";
import { MAX_PROBLEM_COLLECTION_MEMBERS } from "../../../generated/api/MAX_PROBLEM_COLLECTION_MEMBERS";
import { MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS";
import type { CatalogSearchFilter } from "../../../generated/api/CatalogSearchFilter";
import type { ProblemCollectionMemberView } from "../../../generated/api/ProblemCollectionMemberView";
import type { ProblemCollectionReference } from "../../../generated/api/ProblemCollectionReference";
import type { ProblemCollectionRevision } from "../../../generated/api/ProblemCollectionRevision";
import type { ProblemCollectionSummaryView } from "../../../generated/api/ProblemCollectionSummaryView";
import type { SavedProblemSearchReference } from "../../../generated/api/SavedProblemSearchReference";
import type { SavedProblemSearchRevision } from "../../../generated/api/SavedProblemSearchRevision";
import type { SavedProblemSearchView } from "../../../generated/api/SavedProblemSearchView";
import type { CursorPage } from "../contracts";
import {
  DecodeError,
  decodeNonemptyString,
  decodeNullable,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeCatalogProblemSummary } from "./catalog_course";
import { decodeBoundedArray, decodeCursor, field, requireOnlyFields } from "./shared";

const MAX_CATALOG_TEXT_UNICODE_SCALARS = 256;
const MAX_CATALOG_TAXONOMY_FILTERS = 64;
const MAX_CATALOG_FILTER_TEXT_UNICODE_SCALARS = 256;
const MAX_CATALOG_TAXONOMY_PART_UNICODE_SCALARS = 128;
const MAX_SAVED_PROBLEM_SEARCHES_PAGE_ITEMS = 100;
const MAX_NAMED_PROBLEM_COLLECTIONS_PAGE_ITEMS = 100;
const QUESTION_ID_PATTERN = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;

function scalarLength(value: string): number {
  return Array.from(value).length;
}

function decodeCurationTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (title !== title.trim() || scalarLength(title) > MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `a trimmed title no longer than ${MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return title;
}

function decodeReference(value: unknown, path: string, prefix: "PC" | "PS"): string {
  const reference = decodeString(value, path);
  const expression = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  if (!expression.test(reference) || Number(reference.slice(prefix.length + 1)) > 2_147_483_647) {
    throw new DecodeError(path, `a canonical ${prefix} public reference`);
  }
  return reference;
}

function decodeRevision(value: unknown, path: string): string {
  const revision = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(revision) || BigInt(revision) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
  }
  return revision;
}

function decodeFilterText(value: unknown, path: string, maximum: number): string {
  const text = decodeNonemptyString(value, path);
  if (text !== text.trim() || scalarLength(text) > maximum) {
    throw new DecodeError(path, `trimmed text no longer than ${maximum} Unicode scalar values`);
  }
  return text;
}

function decodeCatalogSearchFilter(value: unknown, path: string): CatalogSearchFilter {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "text",
    "bylines",
    "backends",
    "tags",
    "response_families",
    "taxonomy",
    "capabilities",
    "licenses",
    "evidence",
    "used_in_my_courses",
    "authorship",
  ]);
  const decoded = {
    text: decodeNullable(field(record, "text", path), `${path}.text`, (entry, entryPath) =>
      decodeFilterText(entry, entryPath, MAX_CATALOG_TEXT_UNICODE_SCALARS),
    ),
    bylines: decodeBoundedArray(
      field(record, "bylines", path),
      `${path}.bylines`,
      MAX_CATALOG_BYLINE_FILTERS,
      (entry, entryPath) => decodeFilterText(entry, entryPath, 120),
    ),
    backends: decodeBoundedArray(
      field(record, "backends", path),
      `${path}.backends`,
      5,
      (entry, entryPath) =>
        decodeStringEnum(entry, entryPath, ["native", "webwork", "qti", "h5p", "imathas"]),
    ),
    tags: decodeBoundedArray(
      field(record, "tags", path),
      `${path}.tags`,
      MAX_CATALOG_TAG_FILTERS,
      (entry, entryPath) =>
        decodeFilterText(entry, entryPath, MAX_CATALOG_FILTER_TEXT_UNICODE_SCALARS),
    ),
    response_families: decodeBoundedArray(
      field(record, "response_families", path),
      `${path}.response_families`,
      9,
      (entry, entryPath) =>
        decodeStringEnum(entry, entryPath, [
          "numeric",
          "multipleChoice",
          "shortText",
          "multiBlank",
          "matching",
          "ordering",
          "hotspot",
          "fileUpload",
          "externalTool",
        ]),
    ),
    taxonomy: decodeBoundedArray(
      field(record, "taxonomy", path),
      `${path}.taxonomy`,
      MAX_CATALOG_TAXONOMY_FILTERS,
      (entry, entryPath) => {
        const taxonomy = decodeRecord(entry, entryPath);
        requireOnlyFields(taxonomy, entryPath, ["scheme", "code"]);
        return {
          scheme: decodeFilterText(
            field(taxonomy, "scheme", entryPath),
            `${entryPath}.scheme`,
            MAX_CATALOG_TAXONOMY_PART_UNICODE_SCALARS,
          ),
          code: decodeFilterText(
            field(taxonomy, "code", entryPath),
            `${entryPath}.code`,
            MAX_CATALOG_TAXONOMY_PART_UNICODE_SCALARS,
          ),
        };
      },
    ),
    capabilities: decodeBoundedArray(
      field(record, "capabilities", path),
      `${path}.capabilities`,
      8,
      (entry, entryPath) =>
        decodeStringEnum(entry, entryPath, [
          "algorithmicGeneration",
          "clientRendering",
          "serverGrading",
          "partialCredit",
          "hints",
          "perQuestionTiming",
          "printExport",
          "offlinePreview",
        ]),
    ),
    licenses: decodeBoundedArray(
      field(record, "licenses", path),
      `${path}.licenses`,
      6,
      (entry, entryPath) =>
        decodeStringEnum(entry, entryPath, [
          "allRightsReserved",
          "ccBy",
          "ccBySa",
          "ccByNc",
          "cc0",
          "other",
        ]),
    ),
    evidence: decodeStringEnum(field(record, "evidence", path), `${path}.evidence`, [
      "any",
      "available",
      "unavailable",
    ]),
    used_in_my_courses: decodeStringEnum(
      field(record, "used_in_my_courses", path),
      `${path}.used_in_my_courses`,
      ["any", "used"],
    ),
    authorship: decodeStringEnum(field(record, "authorship", path), `${path}.authorship`, [
      "any",
      "authoredByCurrentAccount",
    ]),
  } satisfies CatalogSearchFilter;
  return decoded;
}

function decodeProblemCollectionSummary(
  value: unknown,
  path: string,
): ProblemCollectionSummaryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "kind",
    "title",
    "visibility",
    "revision",
    "access",
  ]);
  return {
    reference: decodeReference(field(record, "reference", path), `${path}.reference`, "PC"),
    kind: decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["favorites", "named"]),
    title: decodeCurationTitle(field(record, "title", path), `${path}.title`),
    visibility: decodeStringEnum(field(record, "visibility", path), `${path}.visibility`, [
      "private",
      "institution",
    ]),
    revision: decodeRevision(field(record, "revision", path), `${path}.revision`),
    access: decodeStringEnum(field(record, "access", path), `${path}.access`, [
      "owner",
      "institutionReader",
    ]),
  };
}

function decodeProblemCollectionMember(value: unknown, path: string): ProblemCollectionMemberView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "summary", "selectionAvailability"]);
  const summary = decodeCatalogProblemSummary(
    field(record, "summary", path),
    `${path}.summary`,
    true,
  );
  const questionId = decodeString(field(record, "questionId", path), `${path}.questionId`);
  if (!QUESTION_ID_PATTERN.test(questionId) || questionId !== summary.questionId) {
    throw new DecodeError(`${path}.questionId`, "the member summary's canonical Question ID");
  }
  return {
    questionId,
    summary,
    selectionAvailability: decodeStringEnum(
      field(record, "selectionAvailability", path),
      `${path}.selectionAvailability`,
      ["available", "retained"],
    ),
  };
}

function decodeSavedProblemSearch(value: unknown, path: string): SavedProblemSearchView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title", "filter", "revision"]);
  return {
    reference: decodeReference(field(record, "reference", path), `${path}.reference`, "PS"),
    title: decodeCurationTitle(field(record, "title", path), `${path}.title`),
    filter: decodeCatalogSearchFilter(field(record, "filter", path), `${path}.filter`),
    revision: decodeRevision(field(record, "revision", path), `${path}.revision`),
  };
}

function decodePage<T>(
  value: unknown,
  path: string,
  maximum: number,
  decodeItem: (entry: unknown, entryPath: string) => T,
): CursorPage<T> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(field(record, "items", path), `${path}.items`, maximum, decodeItem),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}

export function decodeProblemCollectionPage(
  value: unknown,
  path = "response",
): CursorPage<ProblemCollectionSummaryView> {
  return decodePage(
    value,
    path,
    MAX_NAMED_PROBLEM_COLLECTIONS_PAGE_ITEMS,
    decodeProblemCollectionSummary,
  );
}

export function decodeProblemCollectionMemberPage(
  value: unknown,
  path = "response",
): CursorPage<ProblemCollectionMemberView> {
  return decodePage(value, path, MAX_PROBLEM_COLLECTION_MEMBERS, decodeProblemCollectionMember);
}

export function decodeSavedProblemSearchPage(
  value: unknown,
  path = "response",
): CursorPage<SavedProblemSearchView> {
  return decodePage(value, path, MAX_SAVED_PROBLEM_SEARCHES_PAGE_ITEMS, decodeSavedProblemSearch);
}

export function decodeProblemCollectionSummaryView(
  value: unknown,
  path = "response",
): ProblemCollectionSummaryView {
  return decodeProblemCollectionSummary(value, path);
}

export function decodeSavedProblemSearchView(
  value: unknown,
  path = "response",
): SavedProblemSearchView {
  return decodeSavedProblemSearch(value, path);
}

export function decodeSavedProblemSearchFilter(
  value: unknown,
  path = "response",
): CatalogSearchFilter {
  return decodeCatalogSearchFilter(value, path);
}

export function decodeProblemCollectionQuestionIds(
  value: unknown,
  path = "request",
): Array<string> {
  const questionIds = decodeBoundedArray(
    value,
    path,
    MAX_PROBLEM_COLLECTION_MEMBERS,
    (entry, entryPath) => {
      const questionId = decodeString(entry, entryPath);
      if (!QUESTION_ID_PATTERN.test(questionId)) {
        throw new DecodeError(entryPath, "a canonical Question ID");
      }
      return questionId;
    },
  );
  if (new Set(questionIds).size !== questionIds.length) {
    throw new DecodeError(path, "an ordered list of distinct Question IDs");
  }
  return questionIds;
}

export function decodeProblemCurationTitle(value: unknown, path = "request.title"): string {
  return decodeCurationTitle(value, path);
}

export function decodeProblemCollectionReference(
  value: unknown,
  path = "reference",
): ProblemCollectionReference {
  return decodeReference(value, path, "PC");
}

export function decodeSavedProblemSearchReference(
  value: unknown,
  path = "reference",
): SavedProblemSearchReference {
  return decodeReference(value, path, "PS");
}

export function decodeProblemCollectionRevision(
  value: unknown,
  path = "revision",
): ProblemCollectionRevision {
  return decodeRevision(value, path);
}

export function decodeSavedProblemSearchRevision(
  value: unknown,
  path = "revision",
): SavedProblemSearchRevision {
  return decodeRevision(value, path);
}
