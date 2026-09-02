// Strict runtime decoding for the answer-free Question Search facet DTO.

import { MAX_QUESTION_SEARCH_CLASSIFICATION_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_CLASSIFICATION_FACETS";
import { MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS";
import { MAX_QUESTION_SEARCH_TAG_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_TAG_FACETS";
import type { QuestionSearchBackendFacet } from "../../../generated/api/QuestionSearchBackendFacet";
import type { QuestionSearchAuthorFacet } from "../../../generated/api/QuestionSearchAuthorFacet";
import type { QuestionSearchCapabilityFacet } from "../../../generated/api/QuestionSearchCapabilityFacet";
import type { QuestionStatisticsAvailabilityFacet } from "../../../generated/api/QuestionStatisticsAvailabilityFacet";
import type { QuestionSearchQuestionLicenseFacet } from "../../../generated/api/QuestionSearchQuestionLicenseFacet";
import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionTypeFacet } from "../../../generated/api/QuestionTypeFacet";
import type { QuestionSearchFacets } from "../../../generated/api/QuestionSearchFacets";
import type { QuestionSearchTagFacet } from "../../../generated/api/QuestionSearchTagFacet";
import type { QuestionSearchClassificationFacet } from "../../../generated/api/QuestionSearchClassificationFacet";
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
  MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS,
  decodeBoundedArray,
  decodeCapability,
  decodeQuestionClassification,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_QUESTION_SEARCH_BACKEND_FACETS = 5;
const MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS = 8;

function decodeQuestionSearchClassificationFacet(
  value: unknown,
  path: string,
): QuestionSearchClassificationFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["classification", "count"]);
  return {
    classification: decodeQuestionClassification(
      field(record, "classification", path),
      `${path}.classification`,
      true,
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionSearchFacetText(value: unknown, path: string, maximum: number): string {
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

function decodeQuestionSearchAuthorFacet(value: unknown, path: string): QuestionSearchAuthorFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authorName", "count"]);
  return {
    authorName: decodeQuestionSearchFacetText(
      field(record, "authorName", path),
      `${path}.authorName`,
      120,
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionSearchBackendFacet(
  value: unknown,
  path: string,
): QuestionSearchBackendFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["backend", "count"]);
  return {
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "ple",
      "webwork",
      "qti",
      "imathas",
    ]),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionSearchTagFacet(value: unknown, path: string): QuestionSearchTagFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["tag", "count"]);
  return {
    tag: decodeQuestionSearchFacetText(field(record, "tag", path), `${path}.tag`, 256),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionTypeFacet(value: unknown, path: string): QuestionTypeFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionType", "count"]);
  return {
    questionType: decodeStringEnum(field(record, "questionType", path), `${path}.questionType`, [
      "multipleChoice",
      "multipleAnswer",
      "fillInBlank",
      "multipleFillInBlank",
      "numeric",
      "matching",
      "ordering",
      "hotspot",
    ]),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionSearchCapabilityFacet(
  value: unknown,
  path: string,
): QuestionSearchCapabilityFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["capability", "count"]);
  return {
    capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionSearchQuestionLicenseFacet(
  value: unknown,
  path: string,
): QuestionSearchQuestionLicenseFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionLicense", "count"]);
  return {
    questionLicense: decodeStringEnum<QuestionLicense>(
      field(record, "questionLicense", path),
      `${path}.questionLicense`,
      ["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"],
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionStatisticsAvailabilityFacet(
  value: unknown,
  path: string,
): QuestionStatisticsAvailabilityFacet {
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

function decodeQuestionSearchCourseUseFacet(
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
 * Question Search facet projection generated from the Rust contract.
 */
export function decodeQuestionSearchFacets(value: unknown, path: string): QuestionSearchFacets {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "authorNames",
    "backends",
    "tags",
    "questionTypes",
    "classifications",
    "capabilities",
    "questionLicenses",
    "evidence",
    "usedInMyCourses",
  ]);
  return {
    authorNames: decodeBoundedArray(
      field(record, "authorNames", path),
      `${path}.authorNames`,
      MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS,
      decodeQuestionSearchAuthorFacet,
    ),
    backends: decodeBoundedArray(
      field(record, "backends", path),
      `${path}.backends`,
      MAX_QUESTION_SEARCH_BACKEND_FACETS,
      decodeQuestionSearchBackendFacet,
    ),
    tags: decodeBoundedArray(
      field(record, "tags", path),
      `${path}.tags`,
      MAX_QUESTION_SEARCH_TAG_FACETS,
      decodeQuestionSearchTagFacet,
    ),
    questionTypes: decodeBoundedArray(
      field(record, "questionTypes", path),
      `${path}.questionTypes`,
      MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
      decodeQuestionTypeFacet,
    ),
    classifications: decodeBoundedArray(
      field(record, "classifications", path),
      `${path}.classifications`,
      MAX_QUESTION_SEARCH_CLASSIFICATION_FACETS,
      decodeQuestionSearchClassificationFacet,
    ),
    capabilities: decodeBoundedArray(
      field(record, "capabilities", path),
      `${path}.capabilities`,
      MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
      decodeQuestionSearchCapabilityFacet,
    ),
    questionLicenses: decodeBoundedArray(
      field(record, "questionLicenses", path),
      `${path}.questionLicenses`,
      MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS,
      decodeQuestionSearchQuestionLicenseFacet,
    ),
    evidence: decodeQuestionStatisticsAvailabilityFacet(
      field(record, "evidence", path),
      `${path}.evidence`,
    ),
    usedInMyCourses: decodeQuestionSearchCourseUseFacet(
      field(record, "usedInMyCourses", path),
      `${path}.usedInMyCourses`,
    ),
  };
}
