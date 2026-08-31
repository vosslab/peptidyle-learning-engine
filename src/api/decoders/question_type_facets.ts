// Strict runtime decoding for the answer-free catalog-search facet DTO.

import { MAX_QUESTION_SEARCH_TAXONOMY_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_TAXONOMY_FACETS";
import { MAX_QUESTION_SEARCH_BYLINE_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_BYLINE_FACETS";
import { MAX_QUESTION_SEARCH_TAG_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_TAG_FACETS";
import type { QuestionSearchBackendFacet } from "../../../generated/api/QuestionSearchBackendFacet";
import type { QuestionSearchBylineFacet } from "../../../generated/api/QuestionSearchBylineFacet";
import type { QuestionSearchCapabilityFacet } from "../../../generated/api/QuestionSearchCapabilityFacet";
import type { QuestionStatisticsAvailabilityFacet } from "../../../generated/api/QuestionStatisticsAvailabilityFacet";
import type { QuestionSearchLicenseFacet } from "../../../generated/api/QuestionSearchLicenseFacet";
import type { QuestionSearchLicense } from "../../../generated/api/QuestionSearchLicense";
import type { QuestionTypeFacet } from "../../../generated/api/QuestionTypeFacet";
import type { QuestionSearchFacets } from "../../../generated/api/QuestionSearchFacets";
import type { QuestionSearchTagFacet } from "../../../generated/api/QuestionSearchTagFacet";
import type { QuestionSearchTaxonomyFacet } from "../../../generated/api/QuestionSearchTaxonomyFacet";
import type { QuestionSearchCourseUseFacet } from "../../../generated/api/QuestionSearchCourseUseFacet";
import {
  DecodeError,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import {
  MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
  MAX_QUESTION_SEARCH_LICENSE_FACETS,
  decodeBoundedArray,
  decodeCapability,
  decodeTaxonomyTerm,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_QUESTION_SEARCH_BACKEND_FACETS = 5;
const MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS = 8;

function decodeCatalogTaxonomyFacet(value: unknown, path: string): QuestionSearchTaxonomyFacet {
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

function decodeCatalogBylineFacet(value: unknown, path: string): QuestionSearchBylineFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["byline", "count"]);
  return {
    byline: decodeCatalogFacetText(field(record, "byline", path), `${path}.byline`, 120),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogBackendFacet(value: unknown, path: string): QuestionSearchBackendFacet {
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

function decodeCatalogTagFacet(value: unknown, path: string): QuestionSearchTagFacet {
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

function decodeCatalogCapabilityFacet(value: unknown, path: string): QuestionSearchCapabilityFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["capability", "count"]);
  return {
    capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogLicenseFacet(value: unknown, path: string): QuestionSearchLicenseFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["license", "count"]);
  return {
    license: decodeStringEnum<QuestionSearchLicense>(
      field(record, "license", path),
      `${path}.license`,
      ["allRightsReserved", "ccBy", "ccBySa", "ccByNc", "cc0", "other"],
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogEvidenceFacet(value: unknown, path: string): QuestionStatisticsAvailabilityFacet {
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
): QuestionSearchCourseUseFacet {
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
export function decodeQuestionSearchFacets(value: unknown, path: string): QuestionSearchFacets {
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
      MAX_QUESTION_SEARCH_BYLINE_FACETS,
      decodeCatalogBylineFacet,
    ),
    backends: decodeBoundedArray(
      field(record, "backends", path),
      `${path}.backends`,
      MAX_QUESTION_SEARCH_BACKEND_FACETS,
      decodeCatalogBackendFacet,
    ),
    tags: decodeBoundedArray(
      field(record, "tags", path),
      `${path}.tags`,
      MAX_QUESTION_SEARCH_TAG_FACETS,
      decodeCatalogTagFacet,
    ),
    questionTypes: decodeBoundedArray(
      field(record, "questionTypes", path),
      `${path}.questionTypes`,
      MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
      decodeQuestionTypeFacet,
    ),
    taxonomy: decodeBoundedArray(
      field(record, "taxonomy", path),
      `${path}.taxonomy`,
      MAX_QUESTION_SEARCH_TAXONOMY_FACETS,
      decodeCatalogTaxonomyFacet,
    ),
    capabilities: decodeBoundedArray(
      field(record, "capabilities", path),
      `${path}.capabilities`,
      MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
      decodeCatalogCapabilityFacet,
    ),
    licenses: decodeBoundedArray(
      field(record, "licenses", path),
      `${path}.licenses`,
      MAX_QUESTION_SEARCH_LICENSE_FACETS,
      decodeCatalogLicenseFacet,
    ),
    evidence: decodeCatalogEvidenceFacet(field(record, "evidence", path), `${path}.evidence`),
    usedInMyCourses: decodeCatalogUsedInMyCoursesFacet(
      field(record, "usedInMyCourses", path),
      `${path}.usedInMyCourses`,
    ),
  };
}
