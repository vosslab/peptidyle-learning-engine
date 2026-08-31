// Strict runtime decoding for the answer-free catalog-search facet DTO.

import { MAX_CATALOG_TAXONOMY_FACETS } from "../../../generated/api/MAX_CATALOG_TAXONOMY_FACETS";
import { MAX_CATALOG_BYLINE_FACETS } from "../../../generated/api/MAX_CATALOG_BYLINE_FACETS";
import { MAX_CATALOG_TAG_FACETS } from "../../../generated/api/MAX_CATALOG_TAG_FACETS";
import type { CatalogBackendFacet } from "../../../generated/api/CatalogBackendFacet";
import type { CatalogBylineFacet } from "../../../generated/api/CatalogBylineFacet";
import type { CatalogCapabilityFacet } from "../../../generated/api/CatalogCapabilityFacet";
import type { CatalogEvidenceFacet } from "../../../generated/api/CatalogEvidenceFacet";
import type { CatalogLicenseFacet } from "../../../generated/api/CatalogLicenseFacet";
import type { CatalogLicenseValue } from "../../../generated/api/CatalogLicenseValue";
import type { QuestionTypeFacet } from "../../../generated/api/QuestionTypeFacet";
import type { CatalogSearchFacets } from "../../../generated/api/CatalogSearchFacets";
import type { CatalogTagFacet } from "../../../generated/api/CatalogTagFacet";
import type { CatalogTaxonomyFacet } from "../../../generated/api/CatalogTaxonomyFacet";
import type { CatalogUsedInMyCoursesFacet } from "../../../generated/api/CatalogUsedInMyCoursesFacet";
import {
  DecodeError,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import {
  MAX_CATALOG_CAPABILITY_FACETS,
  MAX_CATALOG_LICENSE_FACETS,
  decodeBoundedArray,
  decodeCapability,
  decodeTaxonomyTerm,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_CATALOG_BACKEND_FACETS = 5;
const MAX_CATALOG_QUESTION_TYPE_FACETS = 8;

function decodeCatalogTaxonomyFacet(value: unknown, path: string): CatalogTaxonomyFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["term", "count"]);
  return {
    term: decodeTaxonomyTerm(field(record, "term", path), `${path}.term`, true),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogFacetText(value: unknown, path: string, maximum: number): string {
  const decoded = decodeNonemptyString(value, path);
  if (
    decoded !== decoded.trim() ||
    /[\p{Cc}]/u.test(decoded) ||
    Array.from(decoded).length > maximum
  ) {
    throw new DecodeError(path, `trimmed public text no longer than ${maximum} characters`);
  }
  return decoded;
}

function decodeCatalogBylineFacet(value: unknown, path: string): CatalogBylineFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["byline", "count"]);
  return {
    byline: decodeCatalogFacetText(field(record, "byline", path), `${path}.byline`, 120),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogBackendFacet(value: unknown, path: string): CatalogBackendFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["backend", "count"]);
  return {
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "native",
      "webwork",
      "qti",
      "h5p",
      "imathas",
    ]),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogTagFacet(value: unknown, path: string): CatalogTagFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["tag", "count"]);
  return {
    tag: decodeCatalogFacetText(field(record, "tag", path), `${path}.tag`, 256),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionTypeFacet(
  value: unknown,
  path: string,
): QuestionTypeFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionType", "count"]);
  return {
    questionType: decodeStringEnum(
      field(record, "questionType", path),
      `${path}.questionType`,
      [
        "multipleChoice",
        "multipleAnswer",
        "fillInBlank",
        "multipleFillInBlank",
        "numeric",
        "matching",
        "ordering",
        "hotspot",
      ],
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogCapabilityFacet(value: unknown, path: string): CatalogCapabilityFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["capability", "count"]);
  return {
    capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogLicenseFacet(value: unknown, path: string): CatalogLicenseFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["license", "count"]);
  return {
    license: decodeStringEnum<CatalogLicenseValue>(
      field(record, "license", path),
      `${path}.license`,
      ["allRightsReserved", "ccBy", "ccBySa", "ccByNc", "cc0", "other"],
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogEvidenceFacet(value: unknown, path: string): CatalogEvidenceFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["available", "unavailable"]);
  return {
    available: decodeNonnegativeInteger(field(record, "available", path), `${path}.available`),
    unavailable: decodeNonnegativeInteger(
      field(record, "unavailable", path),
      `${path}.unavailable`,
    ),
  };
}

function decodeCatalogUsedInMyCoursesFacet(
  value: unknown,
  path: string,
): CatalogUsedInMyCoursesFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["used"]);
  return {
    used: decodeNonnegativeInteger(field(record, "used", path), `${path}.used`),
  };
}

/**
 * ASVS 1.5.2 and 2.2.1: strictly decodes only the same-query, answer-free
 * catalog facet projection generated from the Rust contract.
 */
export function decodeCatalogSearchFacets(value: unknown, path: string): CatalogSearchFacets {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "bylines",
    "backends",
    "tags",
    "questionTypes",
    "taxonomy",
    "capabilities",
    "licenses",
    "evidence",
    "usedInMyCourses",
  ]);
  return {
    bylines: decodeBoundedArray(
      field(record, "bylines", path),
      `${path}.bylines`,
      MAX_CATALOG_BYLINE_FACETS,
      decodeCatalogBylineFacet,
    ),
    backends: decodeBoundedArray(
      field(record, "backends", path),
      `${path}.backends`,
      MAX_CATALOG_BACKEND_FACETS,
      decodeCatalogBackendFacet,
    ),
    tags: decodeBoundedArray(
      field(record, "tags", path),
      `${path}.tags`,
      MAX_CATALOG_TAG_FACETS,
      decodeCatalogTagFacet,
    ),
    questionTypes: decodeBoundedArray(
      field(record, "questionTypes", path),
      `${path}.questionTypes`,
      MAX_CATALOG_QUESTION_TYPE_FACETS,
      decodeQuestionTypeFacet,
    ),
    taxonomy: decodeBoundedArray(
      field(record, "taxonomy", path),
      `${path}.taxonomy`,
      MAX_CATALOG_TAXONOMY_FACETS,
      decodeCatalogTaxonomyFacet,
    ),
    capabilities: decodeBoundedArray(
      field(record, "capabilities", path),
      `${path}.capabilities`,
      MAX_CATALOG_CAPABILITY_FACETS,
      decodeCatalogCapabilityFacet,
    ),
    licenses: decodeBoundedArray(
      field(record, "licenses", path),
      `${path}.licenses`,
      MAX_CATALOG_LICENSE_FACETS,
      decodeCatalogLicenseFacet,
    ),
    evidence: decodeCatalogEvidenceFacet(field(record, "evidence", path), `${path}.evidence`),
    usedInMyCourses: decodeCatalogUsedInMyCoursesFacet(
      field(record, "usedInMyCourses", path),
      `${path}.usedInMyCourses`,
    ),
  };
}
