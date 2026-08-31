// catalog_query.ts - one strict request boundary for the production client and boundary tests.

import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import { MAX_CATALOG_BYLINE_FILTERS } from "../../generated/api/MAX_CATALOG_BYLINE_FILTERS";
import { MAX_CATALOG_TAG_FILTERS } from "../../generated/api/MAX_CATALOG_TAG_FILTERS";

const MAX_CATALOG_TEXT_UNICODE_SCALARS = 256;
const MAX_CATALOG_TAXONOMY_FILTERS = 64;
const MAX_CATALOG_PAGE_SIZE = 100;
const MAX_PROBLEM_DISPLAY_REFERENCE_CHARACTERS = 44;
const CATALOG_CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "perQuestionTiming",
  "printExport",
  "offlinePreview",
] as const;
const CATALOG_LICENSES = ["allRightsReserved", "ccBy", "ccBySa", "ccByNc", "cc0", "other"] as const;
const CATALOG_BACKENDS = ["native", "webwork", "qti", "h5p", "imathas"] as const;
const CATALOG_QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const;
const CATALOG_QUERY_FIELDS = [
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
  "cursor",
  "page_size",
] as const;

function catalogEnum(value: string, allowed: ReadonlyArray<string>, fieldName: string): string {
  if (!allowed.includes(value)) {
    throw new Error(`${fieldName} must be a supported catalog value`);
  }
  return value;
}

function catalogFilterText(value: string, fieldName: string, maximum: number): string {
  if (value.trim().length === 0 || Array.from(value).length > maximum) {
    throw new Error(
      `${fieldName} must contain non-whitespace text no longer than ${maximum} characters`,
    );
  }
  return value;
}

function normalizedCatalogFilterText(value: string, fieldName: string, maximum: number): string {
  const normalized = value.trim().split(/\s+/u).join(" ").toLowerCase();
  if (normalized.length === 0 || Array.from(normalized).length > maximum) {
    throw new Error(
      `${fieldName} must contain non-whitespace text no longer than ${maximum} characters`,
    );
  }
  return normalized;
}

function boundedCatalogFilterValues(
  values: ReadonlyArray<string>,
  maximum: number,
  fieldName: string,
): void {
  if (values.length > maximum) {
    throw new Error(`${fieldName} must contain at most ${maximum} entries`);
  }
}

function catalogCursor(value: string): string {
  if (value.length === 0) {
    throw new Error("catalog cursor must not be empty");
  }
  return value;
}

/**
 * ASVS 2.2.1: validates and serializes the only bounded, allowlisted
 * cursor-based catalog-search request shape.
 */
export function catalogSearchPath(query: CatalogSearchQuery): string {
  for (const field of Object.keys(query)) {
    if (!CATALOG_QUERY_FIELDS.includes(field as (typeof CATALOG_QUERY_FIELDS)[number])) {
      throw new Error(`catalog search query contains unknown field: ${field}`);
    }
  }
  const parameters = new URLSearchParams();
  if (query.text !== null) {
    parameters.set(
      "text",
      catalogFilterText(query.text, "catalog text", MAX_CATALOG_TEXT_UNICODE_SCALARS),
    );
  }
  boundedCatalogFilterValues(query.bylines, MAX_CATALOG_BYLINE_FILTERS, "catalog bylines");
  for (const byline of query.bylines) {
    parameters.append("bylines", normalizedCatalogFilterText(byline, "catalog byline", 120));
  }
  boundedCatalogFilterValues(query.backends, CATALOG_BACKENDS.length, "catalog backends");
  for (const backend of query.backends) {
    parameters.append("backends", catalogEnum(backend, CATALOG_BACKENDS, "catalog backend"));
  }
  boundedCatalogFilterValues(query.tags, MAX_CATALOG_TAG_FILTERS, "catalog tags");
  for (const tag of query.tags) {
    parameters.append("tags", normalizedCatalogFilterText(tag, "catalog tag", 256));
  }
  boundedCatalogFilterValues(
    query.question_types,
    CATALOG_QUESTION_TYPES.length,
    "catalog question_types",
  );
  for (const questionType of query.question_types) {
    parameters.append(
      "question_types",
      catalogEnum(questionType, CATALOG_QUESTION_TYPES, "catalog Question Type"),
    );
  }
  if (query.taxonomy.length > MAX_CATALOG_TAXONOMY_FILTERS) {
    throw new Error(
      `catalog taxonomy filters must contain at most ${MAX_CATALOG_TAXONOMY_FILTERS} entries`,
    );
  }
  for (const taxonomy of query.taxonomy) {
    const scheme = catalogFilterText(taxonomy.scheme, "catalog taxonomy scheme", 128);
    const code = catalogFilterText(taxonomy.code, "catalog taxonomy code", 128);
    parameters.append("taxonomy", `${scheme}:${code}`);
  }
  if (query.capabilities.length > CATALOG_CAPABILITIES.length) {
    throw new Error("catalog capabilities must contain at most the supported capability count");
  }
  for (const capability of query.capabilities) {
    parameters.append(
      "capabilities",
      catalogEnum(capability, CATALOG_CAPABILITIES, "catalog capability"),
    );
  }
  if (query.licenses.length > CATALOG_LICENSES.length) {
    throw new Error("catalog licenses must contain at most the supported license count");
  }
  for (const license of query.licenses) {
    parameters.append("licenses", catalogEnum(license, CATALOG_LICENSES, "catalog license"));
  }
  const evidence = catalogEnum(
    query.evidence,
    ["any", "available", "unavailable"],
    "catalog evidence",
  );
  if (evidence !== "any") {
    parameters.set("evidence", evidence);
  }
  const usedInMyCourses = catalogEnum(
    query.used_in_my_courses,
    ["any", "used"],
    "catalog used_in_my_courses",
  );
  if (usedInMyCourses !== "any") {
    parameters.set("used_in_my_courses", usedInMyCourses);
  }
  const authorship = catalogEnum(
    query.authorship,
    ["any", "authoredByCurrentAccount"],
    "catalog authorship scope",
  );
  // Keep the current visible source explicit in every cursor-bound request.
  // `any` is a closed scope, not an omitted identity fallback.
  parameters.set("authorship", authorship);
  if (query.cursor !== null) {
    parameters.set("cursor", catalogCursor(query.cursor));
  }
  if (query.page_size !== null) {
    if (
      !Number.isSafeInteger(query.page_size) ||
      query.page_size < 1 ||
      query.page_size > MAX_CATALOG_PAGE_SIZE
    ) {
      throw new Error(
        `catalog page_size must be a safe integer between 1 and ${MAX_CATALOG_PAGE_SIZE}`,
      );
    }
    parameters.set("page_size", String(query.page_size));
  }
  const suffix = parameters.size === 0 ? "" : `?${parameters.toString()}`;
  return `/api/problems/search${suffix}`;
}

/** Serializes the bounded copyable problem locator without interpreting its server-owned syntax. */
export function catalogProblemReferencePath(displayReference: string): string {
  const reference = displayReference.trim();
  if (
    reference.length === 0 ||
    Array.from(reference).length > MAX_PROBLEM_DISPLAY_REFERENCE_CHARACTERS
  ) {
    throw new Error("problem reference must be 1 to 44 characters");
  }
  return `/api/problems/by-id/${encodeURIComponent(reference)}`;
}
