// Strict browser decoding for private Instructor Question Curation APIs.

import { MAX_CATALOG_BYLINE_FILTERS } from "../../../generated/api/MAX_CATALOG_BYLINE_FILTERS";
import { MAX_CATALOG_TAG_FILTERS } from "../../../generated/api/MAX_CATALOG_TAG_FILTERS";
import { MAX_QUESTION_FOLDER_MEMBERS } from "../../../generated/api/MAX_QUESTION_FOLDER_MEMBERS";
import { MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS";
import type { QuestionSearchFilter } from "../../../generated/api/QuestionSearchFilter";
import type { QuestionFolderEntryView } from "../../../generated/api/QuestionFolderEntryView";
import type { QuestionFolderReference } from "../../../generated/api/QuestionFolderReference";
import type { QuestionFolderEditNumber } from "../../../generated/api/QuestionFolderEditNumber";
import type { QuestionFolderSummaryView } from "../../../generated/api/QuestionFolderSummaryView";
import type { SavedQuestionSearchReference } from "../../../generated/api/SavedQuestionSearchReference";
import type { SavedQuestionSearchEditNumber } from "../../../generated/api/SavedQuestionSearchEditNumber";
import type { SavedQuestionSearchView } from "../../../generated/api/SavedQuestionSearchView";
import type { CursorPage } from "../contracts";
import {
  DecodeError,
  decodeNonemptyString,
  decodeNullable,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeQuestionSummary } from "./question_library";
import {
  decodeBoundedArray,
  decodeCursor,
  decodeQuestionVersionAvailability,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_CATALOG_TEXT_UNICODE_SCALARS = 256;
const MAX_CATALOG_TAXONOMY_FILTERS = 64;
const MAX_CATALOG_FILTER_TEXT_UNICODE_SCALARS = 256;
const MAX_CATALOG_TAXONOMY_PART_UNICODE_SCALARS = 128;
const MAX_SAVED_QUESTION_SEARCHES_PAGE_ITEMS = 100;
const MAX_NAMED_QUESTION_FOLDERS_PAGE_ITEMS = 100;
const QUESTION_ID_PATTERN = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;

function scalarLength(value: string): number {
  return Array.from(value).length;
}

function decodeCurationTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (title !== title.trim() || scalarLength(title) > MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `a trimmed title no longer than ${MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return title;
}

function decodeReference(value: unknown, path: string, prefix: "QC" | "QS"): string {
  const reference = decodeString(value, path);
  const expression = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  if (!expression.test(reference) || Number(reference.slice(prefix.length + 1)) > 2_147_483_647) {
    throw new DecodeError(path, `a canonical ${prefix} public reference`);
  }
  return reference;
}

function decodeEditNumber(value: unknown, path: string): string {
  const editNumber = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(editNumber) || BigInt(editNumber) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint edit number");
  }
  return editNumber;
}

function decodeFilterText(value: unknown, path: string, maximum: number): string {
  const text = decodeNonemptyString(value, path);
  if (text !== text.trim() || scalarLength(text) > maximum) {
    throw new DecodeError(path, `trimmed text no longer than ${maximum} Unicode scalar values`);
  }
  return text;
}

function decodeCatalogSearchFilter(value: unknown, path: string): QuestionSearchFilter {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "text",
    "bylines",
    "backends",
    "tags",
    "question_types",
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
    question_types: decodeBoundedArray(
      field(record, "question_types", path),
      `${path}.question_types`,
      8,
      (entry, entryPath) =>
        decodeStringEnum(entry, entryPath, [
          "multipleChoice",
          "multipleAnswer",
          "fillInBlank",
          "multipleFillInBlank",
          "numeric",
          "matching",
          "ordering",
          "hotspot",
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
  } satisfies QuestionSearchFilter;
  return decoded;
}

function decodeQuestionFolderSummary(
  value: unknown,
  path: string,
): QuestionFolderSummaryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "title",
    "editNumber",
  ]);
  return {
    reference: decodeReference(field(record, "reference", path), `${path}.reference`, "QC"),
    title: decodeCurationTitle(field(record, "title", path), `${path}.title`),
    editNumber: decodeEditNumber(field(record, "editNumber", path), `${path}.editNumber`),
  };
}

function decodeQuestionFolderEntry(value: unknown, path: string): QuestionFolderEntryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "summary", "questionVersionAvailability"]);
  const summary = decodeQuestionSummary(
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
    questionVersionAvailability: decodeQuestionVersionAvailability(
      field(record, "questionVersionAvailability", path),
      `${path}.questionVersionAvailability`,
      true,
    ),
  };
}

function decodeSavedQuestionSearch(value: unknown, path: string): SavedQuestionSearchView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title", "filter", "editNumber"]);
  return {
    reference: decodeReference(field(record, "reference", path), `${path}.reference`, "QS"),
    title: decodeCurationTitle(field(record, "title", path), `${path}.title`),
    filter: decodeCatalogSearchFilter(field(record, "filter", path), `${path}.filter`),
    editNumber: decodeEditNumber(field(record, "editNumber", path), `${path}.editNumber`),
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

export function decodeQuestionFolderPage(
  value: unknown,
  path = "response",
): CursorPage<QuestionFolderSummaryView> {
  return decodePage(
    value,
    path,
    MAX_NAMED_QUESTION_FOLDERS_PAGE_ITEMS,
    decodeQuestionFolderSummary,
  );
}

export function decodeQuestionFolderEntryPage(
  value: unknown,
  path = "response",
): CursorPage<QuestionFolderEntryView> {
  return decodePage(value, path, MAX_QUESTION_FOLDER_MEMBERS, decodeQuestionFolderEntry);
}

export function decodeSavedQuestionSearchPage(
  value: unknown,
  path = "response",
): CursorPage<SavedQuestionSearchView> {
  return decodePage(value, path, MAX_SAVED_QUESTION_SEARCHES_PAGE_ITEMS, decodeSavedQuestionSearch);
}

export function decodeQuestionFolderSummaryView(
  value: unknown,
  path = "response",
): QuestionFolderSummaryView {
  return decodeQuestionFolderSummary(value, path);
}

export function decodeSavedQuestionSearchView(
  value: unknown,
  path = "response",
): SavedQuestionSearchView {
  return decodeSavedQuestionSearch(value, path);
}

export function decodeSavedQuestionSearchFilter(
  value: unknown,
  path = "response",
): QuestionSearchFilter {
  return decodeCatalogSearchFilter(value, path);
}

export function decodeQuestionFolderQuestionIds(
  value: unknown,
  path = "request",
): Array<string> {
  const questionIds = decodeBoundedArray(
    value,
    path,
    MAX_QUESTION_FOLDER_MEMBERS,
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

export function decodeQuestionCurationTitle(value: unknown, path = "request.title"): string {
  return decodeCurationTitle(value, path);
}

export function decodeQuestionFolderReference(
  value: unknown,
  path = "reference",
): QuestionFolderReference {
  return decodeReference(value, path, "QC");
}

export function decodeSavedQuestionSearchReference(
  value: unknown,
  path = "reference",
): SavedQuestionSearchReference {
  return decodeReference(value, path, "QS");
}

export function decodeQuestionFolderEditNumber(
  value: unknown,
  path = "editNumber",
): QuestionFolderEditNumber {
  return decodeEditNumber(value, path);
}

export function decodeSavedQuestionSearchEditNumber(
  value: unknown,
  path = "editNumber",
): SavedQuestionSearchEditNumber {
  return decodeEditNumber(value, path);
}
