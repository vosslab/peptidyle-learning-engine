// Strict runtime decoding for the answer-free Question Search facet DTO.

import { MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS";
import { MAX_QUESTION_SEARCH_TAG_FACETS } from "../../../generated/api/MAX_QUESTION_SEARCH_TAG_FACETS";
import type { QuestionSearchBackendFacet } from "../../../generated/api/QuestionSearchBackendFacet";
import type { QuestionSearchAuthorFacet } from "../../../generated/api/QuestionSearchAuthorFacet";
import type { QuestionSearchCapabilityFacet } from "../../../generated/api/QuestionSearchCapabilityFacet";
import type { QuestionSearchQuestionLicenseFacet } from "../../../generated/api/QuestionSearchQuestionLicenseFacet";
import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionTypeFacet } from "../../../generated/api/QuestionTypeFacet";
import type { QuestionSearchFacets } from "../../../generated/api/QuestionSearchFacets";
import type { QuestionSearchTagFacet } from "../../../generated/api/QuestionSearchTagFacet";
import type { QuestionSearchSubjectFacet } from "../../../generated/api/QuestionSearchSubjectFacet";
import type { QuestionSearchTopicFacet } from "../../../generated/api/QuestionSearchTopicFacet";
import type { QuestionSearchCourseUseFacet } from "../../../generated/api/QuestionSearchCourseUseFacet";
import type { QuestionSearchBloomCognitiveProcessFacet } from "../../../generated/api/QuestionSearchBloomCognitiveProcessFacet";
import type { QuestionSearchBloomKnowledgeDimensionFacet } from "../../../generated/api/QuestionSearchBloomKnowledgeDimensionFacet";
import {
  DecodeError,
  decodeBoolean,
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
  field,
  requireOnlyFields,
} from "./shared";
import { BLOOM_COGNITIVE_PROCESSES, BLOOM_KNOWLEDGE_DIMENSIONS } from "./bloom_classification";

const MAX_QUESTION_SEARCH_BACKEND_FACETS = 5;
export const MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS = 8;

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

function decodeQuestionSearchSubjectFacet(
  value: unknown,
  path: string,
): QuestionSearchSubjectFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["subject", "count"]);
  return {
    subject: decodeQuestionSearchFacetText(field(record, "subject", path), `${path}.subject`, 256),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeQuestionSearchTopicFacet(value: unknown, path: string): QuestionSearchTopicFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["topic", "count"]);
  return {
    topic: decodeQuestionSearchFacetText(field(record, "topic", path), `${path}.topic`, 256),
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

function decodeBloomCognitiveProcessFacet(
  value: unknown,
  path: string,
): QuestionSearchBloomCognitiveProcessFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["cognitiveProcess", "count"]);
  return {
    cognitiveProcess: decodeStringEnum(
      field(record, "cognitiveProcess", path),
      `${path}.cognitiveProcess`,
      BLOOM_COGNITIVE_PROCESSES,
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeBloomKnowledgeDimensionFacet(
  value: unknown,
  path: string,
): QuestionSearchBloomKnowledgeDimensionFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["knowledgeDimension", "count"]);
  return {
    knowledgeDimension: decodeStringEnum(
      field(record, "knowledgeDimension", path),
      `${path}.knowledgeDimension`,
      BLOOM_KNOWLEDGE_DIMENSIONS,
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeBloomCognitiveProcessFacets(
  value: unknown,
  path: string,
): Array<QuestionSearchBloomCognitiveProcessFacet> {
  const facets = decodeBoundedArray(
    value,
    path,
    BLOOM_COGNITIVE_PROCESSES.length,
    decodeBloomCognitiveProcessFacet,
  );
  if (
    facets.length !== BLOOM_COGNITIVE_PROCESSES.length ||
    facets.some((facet, index) => facet.cognitiveProcess !== BLOOM_COGNITIVE_PROCESSES[index])
  ) {
    throw new DecodeError(path, "all Bloom Cognitive Processes in teaching-guide order");
  }
  return facets;
}

function decodeBloomKnowledgeDimensionFacets(
  value: unknown,
  path: string,
): Array<QuestionSearchBloomKnowledgeDimensionFacet> {
  const facets = decodeBoundedArray(
    value,
    path,
    BLOOM_KNOWLEDGE_DIMENSIONS.length,
    decodeBloomKnowledgeDimensionFacet,
  );
  if (
    facets.length !== BLOOM_KNOWLEDGE_DIMENSIONS.length ||
    facets.some((facet, index) => facet.knowledgeDimension !== BLOOM_KNOWLEDGE_DIMENSIONS[index])
  ) {
    throw new DecodeError(path, "all Bloom Knowledge Dimensions in teaching-guide order");
  }
  return facets;
}

/**
 * ASVS 1.5.2 and 2.2.1: strictly decodes only the same-query, answer-free
 * Question Search facets generated from the Rust contract.
 */
export function decodeQuestionSearchFacets(value: unknown, path: string): QuestionSearchFacets {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "authorNames",
    "authorNamesTruncated",
    "backends",
    "tags",
    "tagsTruncated",
    "subjects",
    "subjectsTruncated",
    "topics",
    "topicsTruncated",
    "questionTypes",
    "capabilities",
    "questionLicenses",
    "usedInMyCourses",
    "bloomCognitiveProcesses",
    "bloomKnowledgeDimensions",
  ]);
  return {
    authorNames: decodeBoundedArray(
      field(record, "authorNames", path),
      `${path}.authorNames`,
      MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS,
      decodeQuestionSearchAuthorFacet,
    ),
    authorNamesTruncated: decodeBoolean(
      field(record, "authorNamesTruncated", path),
      `${path}.authorNamesTruncated`,
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
    tagsTruncated: decodeBoolean(field(record, "tagsTruncated", path), `${path}.tagsTruncated`),
    subjects: decodeBoundedArray(
      field(record, "subjects", path),
      `${path}.subjects`,
      MAX_QUESTION_SEARCH_TAG_FACETS,
      decodeQuestionSearchSubjectFacet,
    ),
    subjectsTruncated: decodeBoolean(
      field(record, "subjectsTruncated", path),
      `${path}.subjectsTruncated`,
    ),
    topics: decodeBoundedArray(
      field(record, "topics", path),
      `${path}.topics`,
      MAX_QUESTION_SEARCH_TAG_FACETS,
      decodeQuestionSearchTopicFacet,
    ),
    topicsTruncated: decodeBoolean(
      field(record, "topicsTruncated", path),
      `${path}.topicsTruncated`,
    ),
    questionTypes: decodeBoundedArray(
      field(record, "questionTypes", path),
      `${path}.questionTypes`,
      MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
      decodeQuestionTypeFacet,
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
    usedInMyCourses: decodeQuestionSearchCourseUseFacet(
      field(record, "usedInMyCourses", path),
      `${path}.usedInMyCourses`,
    ),
    bloomCognitiveProcesses: decodeBloomCognitiveProcessFacets(
      field(record, "bloomCognitiveProcesses", path),
      `${path}.bloomCognitiveProcesses`,
    ),
    bloomKnowledgeDimensions: decodeBloomKnowledgeDimensionFacets(
      field(record, "bloomKnowledgeDimensions", path),
      `${path}.bloomKnowledgeDimensions`,
    ),
  };
}
