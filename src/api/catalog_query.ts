// catalog_query.ts - one strict request boundary shared by live and mock catalog clients.

import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";

const MAX_CATALOG_TEXT_UNICODE_SCALARS = 256;
const MAX_CATALOG_TAXONOMY_FILTERS = 64;
const MAX_CATALOG_PAGE_SIZE = 100;
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

function catalogCursor(value: string): string {
  if (value.length === 0) {
    throw new Error("catalog cursor must not be empty");
  }
  return value;
}

/** Validates and serializes the only cursor-based catalog search request shape. */
export function catalogSearchPath(query: CatalogSearchQuery): string {
  const parameters = new URLSearchParams();
  if (query.text !== null) {
    parameters.set(
      "text",
      catalogFilterText(query.text, "catalog text", MAX_CATALOG_TEXT_UNICODE_SCALARS),
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
  const statistics = catalogEnum(
    query.statistics,
    ["any", "available", "unavailable"],
    "catalog statistics",
  );
  if (statistics !== "any") {
    parameters.set("statistics", statistics);
  }
  if (query.cursor !== null) {
    parameters.set("cursor", catalogCursor(query.cursor));
  }
  if (query.pageSize !== null) {
    if (
      !Number.isSafeInteger(query.pageSize) ||
      query.pageSize < 1 ||
      query.pageSize > MAX_CATALOG_PAGE_SIZE
    ) {
      throw new Error(
        `catalog pageSize must be a safe integer between 1 and ${MAX_CATALOG_PAGE_SIZE}`,
      );
    }
    parameters.set("pageSize", String(query.pageSize));
  }
  const suffix = parameters.size === 0 ? "" : `?${parameters.toString()}`;
  return `/api/problems/search${suffix}`;
}
