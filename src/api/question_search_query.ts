// question_search_query.ts - one strict Question Library search boundary for the production client and tests.

import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import { MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS } from "../../generated/api/MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS";
import { MAX_QUESTION_SEARCH_TAG_FILTERS } from "../../generated/api/MAX_QUESTION_SEARCH_TAG_FILTERS";

const MAX_QUESTION_SEARCH_TEXT_UNICODE_SCALARS = 256;
const MAX_QUESTION_SEARCH_PAGE_SIZE = 100;
const MAX_QUESTION_ID_CHARACTERS = 44;
const QUESTION_SEARCH_CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "questionAttemptTimeLimit",
  "printExport",
  "offlinePreview",
] as const;
const QUESTION_LICENSES = ["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"] as const;
const QUESTION_SEARCH_BACKENDS = ["ple", "webwork", "qti", "imathas"] as const;
const QUESTION_SEARCH_QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const;
const QUESTION_SEARCH_QUERY_FIELDS = [
  "text",
  "author_names",
  "backends",
  "tags",
  "question_types",
  "capabilities",
  "question_licenses",
  "evidence",
  "used_in_my_courses",
  "authorship",
  "cursor",
  "page_size",
] as const;

function questionSearchEnum(
  value: string,
  allowed: ReadonlyArray<string>,
  fieldName: string,
): string {
  if (!allowed.includes(value)) {
    throw new Error(`${fieldName} must be a supported Question Library value`);
  }
  return value;
}

function questionSearchFilterText(value: string, fieldName: string, maximum: number): string {
  if (value.trim().length === 0 || Array.from(value).length > maximum) {
    throw new Error(
      `${fieldName} must contain non-whitespace text no longer than ${maximum} characters`,
    );
  }
  return value;
}

function normalizedQuestionSearchFilterText(
  value: string,
  fieldName: string,
  maximum: number,
): string {
  const normalized = value.trim().split(/\s+/u).join(" ").toLowerCase();
  if (normalized.length === 0 || Array.from(normalized).length > maximum) {
    throw new Error(
      `${fieldName} must contain non-whitespace text no longer than ${maximum} characters`,
    );
  }
  return normalized;
}

function boundedQuestionSearchFilterValues(
  values: ReadonlyArray<string>,
  maximum: number,
  fieldName: string,
): void {
  if (values.length > maximum) {
    throw new Error(`${fieldName} must contain at most ${maximum} entries`);
  }
}

function questionSearchCursor(value: string): string {
  if (value.length === 0) {
    throw new Error("Question Library cursor must not be empty");
  }
  return value;
}

/**
 * ASVS 2.2.1: validates and serializes the only bounded, allowlisted
 * cursor-based Question Library search request shape.
 */
export function questionSearchPath(query: QuestionSearchRequest): string {
  for (const field of Object.keys(query)) {
    if (
      !QUESTION_SEARCH_QUERY_FIELDS.includes(field as (typeof QUESTION_SEARCH_QUERY_FIELDS)[number])
    ) {
      throw new Error(`Question Library search query contains unknown field: ${field}`);
    }
  }
  const parameters = new URLSearchParams();
  if (query.text !== null) {
    parameters.set(
      "text",
      questionSearchFilterText(
        query.text,
        "Question Library text",
        MAX_QUESTION_SEARCH_TEXT_UNICODE_SCALARS,
      ),
    );
  }
  boundedQuestionSearchFilterValues(
    query.author_names,
    MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS,
    "Question Library author names",
  );
  for (const authorName of query.author_names) {
    parameters.append(
      "author_names",
      normalizedQuestionSearchFilterText(authorName, "Question Library author name", 120),
    );
  }
  boundedQuestionSearchFilterValues(
    query.backends,
    QUESTION_SEARCH_BACKENDS.length,
    "Question Library backends",
  );
  for (const backend of query.backends) {
    parameters.append(
      "backends",
      questionSearchEnum(backend, QUESTION_SEARCH_BACKENDS, "Question Library backend"),
    );
  }
  boundedQuestionSearchFilterValues(
    query.tags,
    MAX_QUESTION_SEARCH_TAG_FILTERS,
    "Question Library tags",
  );
  for (const tag of query.tags) {
    parameters.append("tags", normalizedQuestionSearchFilterText(tag, "Question Library tag", 256));
  }
  boundedQuestionSearchFilterValues(
    query.question_types,
    QUESTION_SEARCH_QUESTION_TYPES.length,
    "Question Library question_types",
  );
  for (const questionType of query.question_types) {
    parameters.append(
      "question_types",
      questionSearchEnum(
        questionType,
        QUESTION_SEARCH_QUESTION_TYPES,
        "Question Library Question Type",
      ),
    );
  }
  if (query.capabilities.length > QUESTION_SEARCH_CAPABILITIES.length) {
    throw new Error(
      "Question Library capabilities must contain at most the supported capability count",
    );
  }
  for (const capability of query.capabilities) {
    parameters.append(
      "capabilities",
      questionSearchEnum(capability, QUESTION_SEARCH_CAPABILITIES, "Question Library capability"),
    );
  }
  if (query.question_licenses.length > QUESTION_LICENSES.length) {
    throw new Error("Question Library Question Licenses must contain at most the supported count");
  }
  for (const questionLicense of query.question_licenses) {
    parameters.append(
      "question_licenses",
      questionSearchEnum(questionLicense, QUESTION_LICENSES, "Question Library Question License"),
    );
  }
  const evidence = questionSearchEnum(
    query.evidence,
    ["any", "available", "unavailable"],
    "Question Library evidence",
  );
  if (evidence !== "any") {
    parameters.set("evidence", evidence);
  }
  const usedInMyCourses = questionSearchEnum(
    query.used_in_my_courses,
    ["any", "used"],
    "Question Library used_in_my_courses",
  );
  if (usedInMyCourses !== "any") {
    parameters.set("used_in_my_courses", usedInMyCourses);
  }
  const authorship = questionSearchEnum(
    query.authorship,
    ["any", "authoredByCurrentAccount"],
    "Question Library authorship scope",
  );
  // Keep the current visible source explicit in every cursor-bound request.
  // `any` is a closed scope, not an omitted identity fallback.
  parameters.set("authorship", authorship);
  if (query.cursor !== null) {
    parameters.set("cursor", questionSearchCursor(query.cursor));
  }
  if (query.page_size !== null) {
    if (
      !Number.isSafeInteger(query.page_size) ||
      query.page_size < 1 ||
      query.page_size > MAX_QUESTION_SEARCH_PAGE_SIZE
    ) {
      throw new Error(
        `Question Library page_size must be a safe integer between 1 and ${MAX_QUESTION_SEARCH_PAGE_SIZE}`,
      );
    }
    parameters.set("page_size", String(query.page_size));
  }
  const suffix = parameters.size === 0 ? "" : `?${parameters.toString()}`;
  return `/api/questions/search${suffix}`;
}

/** Serializes the bounded copyable Question ID without interpreting its server-owned syntax. */
export function questionReferencePath(displayReference: string): string {
  const reference = displayReference.trim();
  if (reference.length === 0 || Array.from(reference).length > MAX_QUESTION_ID_CHARACTERS) {
    throw new Error("Question ID must be 1 to 44 characters");
  }
  return `/api/questions/by-id/${encodeURIComponent(reference)}`;
}
