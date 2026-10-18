// library_object_search_query.ts - strict Library Object search request serialization.

import type { LibraryObjectSearchRequest } from "../../generated/api/LibraryObjectSearchRequest";
import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import { MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS } from "../../generated/api/MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS";
import { MAX_QUESTION_SEARCH_TAG_FILTERS } from "../../generated/api/MAX_QUESTION_SEARCH_TAG_FILTERS";
import { MAX_DISCOVERY_PAGE_SIZE } from "../../generated/api/MAX_DISCOVERY_PAGE_SIZE";
import { validateCanonicalPublicId, validateCanonicalQuestionIdSyntax } from "../question_id";
import { appendLibraryClassificationParameters } from "./library_classification_filter";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "./decoders/bloom_classification";
import { PRODUCTION_QUESTION_BACKENDS } from "./decoders/shared";

const MAX_QUESTION_SEARCH_TEXT_UNICODE_SCALARS = 256;
const MAX_QUESTION_SEARCH_PAGE_SIZE = MAX_DISCOVERY_PAGE_SIZE;
const MAX_QUESTION_SEARCH_EXCLUDED_QUESTION_IDS = 1024;
const LIBRARY_OBJECT_SEARCH_CAPABILITIES = [
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
const LIBRARY_OBJECT_SEARCH_QUERY_FIELDS = [
  "kind",
  "questions",
  "owner_account_id",
  "text",
  "author_names",
  "backends",
  "tags",
  "subjects",
  "topics",
  "discipline_uuids",
  "subject_uuids",
  "topic_uuids",
  "subtopic_uuids",
  "cross_discipline",
  "bloom_cognitive_processes",
  "bloom_knowledge_dimensions",
  "question_types",
  "capabilities",
  "question_licenses",
  "authorship",
  "sort",
  "excluded_question_ids",
  // Facet choice enumeration has its own independent continuation cursor.
  "cursor",
  "page_number",
  "page_size",
] as const;

function libraryObjectSearchEnum(
  value: string,
  allowed: ReadonlyArray<string>,
  fieldName: string,
): string {
  if (!allowed.includes(value)) {
    throw new Error(`${fieldName} must be a supported Question Library value`);
  }
  return value;
}

function libraryObjectSearchFilterText(value: string, fieldName: string, maximum: number): string {
  if (value.trim().length === 0 || Array.from(value).length > maximum) {
    throw new Error(
      `${fieldName} must contain non-whitespace text no longer than ${maximum} characters`,
    );
  }
  return value;
}

function normalizedLibraryObjectSearchFilterText(
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

function boundedLibraryObjectSearchFilterValues(
  values: ReadonlyArray<string>,
  maximum: number,
  fieldName: string,
): void {
  if (values.length > maximum) {
    throw new Error(`${fieldName} must contain at most ${maximum} entries`);
  }
}

/**
 * ASVS 2.2.1: validates and serializes the only bounded, allowlisted
 * numbered Library Object search request shape.
 */
function libraryObjectSearchParameters(query: LibraryObjectSearchRequest): URLSearchParams {
  for (const field of Object.keys(query)) {
    if (
      !LIBRARY_OBJECT_SEARCH_QUERY_FIELDS.includes(
        field as (typeof LIBRARY_OBJECT_SEARCH_QUERY_FIELDS)[number],
      )
    ) {
      throw new Error(`Question Library search query contains unknown field: ${field}`);
    }
  }
  const parameters = new URLSearchParams();
  appendLibraryClassificationParameters(parameters, query);
  parameters.set(
    "kind",
    libraryObjectSearchEnum(query.kind, ["questions", "pools", "both"], "Question Library kind"),
  );
  parameters.set(
    "questions",
    libraryObjectSearchEnum(query.questions, ["all", "inNoPool"], "Published Question filter"),
  );
  if (query.owner_account_id !== null) {
    const ownerAccountId = validateCanonicalPublicId("account", query.owner_account_id);
    if (ownerAccountId === null) {
      throw new Error("Question Library owner_account_id must be a canonical Account ID");
    }
    parameters.set("owner_account_id", ownerAccountId);
  }
  if (query.text !== null) {
    parameters.set(
      "text",
      libraryObjectSearchFilterText(
        query.text,
        "Question Library text",
        MAX_QUESTION_SEARCH_TEXT_UNICODE_SCALARS,
      ),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.author_names,
    MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS,
    "Question Library author names",
  );
  for (const authorName of query.author_names) {
    parameters.append(
      "author_names",
      normalizedLibraryObjectSearchFilterText(authorName, "Question Library author name", 120),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.backends,
    PRODUCTION_QUESTION_BACKENDS.length,
    "Question Library backends",
  );
  for (const backend of query.backends) {
    parameters.append(
      "backends",
      libraryObjectSearchEnum(backend, PRODUCTION_QUESTION_BACKENDS, "Question Library backend"),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.tags,
    MAX_QUESTION_SEARCH_TAG_FILTERS,
    "Question Library tags",
  );
  for (const tag of query.tags) {
    parameters.append(
      "tags",
      normalizedLibraryObjectSearchFilterText(tag, "Question Library tag", 256),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.subjects,
    MAX_QUESTION_SEARCH_TAG_FILTERS,
    "Question Library subjects",
  );
  for (const subject of query.subjects) {
    parameters.append(
      "subjects",
      normalizedLibraryObjectSearchFilterText(subject, "Question Library subject", 256),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.topics,
    MAX_QUESTION_SEARCH_TAG_FILTERS,
    "Question Library topics",
  );
  for (const topic of query.topics) {
    parameters.append(
      "topics",
      normalizedLibraryObjectSearchFilterText(topic, "Question Library topic", 256),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.bloom_cognitive_processes,
    BLOOM_COGNITIVE_PROCESSES.length,
    "Question Library Bloom Cognitive Process",
  );
  for (const value of query.bloom_cognitive_processes) {
    parameters.append(
      "bloom_cognitive_processes",
      libraryObjectSearchEnum(
        value,
        BLOOM_COGNITIVE_PROCESSES,
        "Question Library Bloom Cognitive Process",
      ),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.bloom_knowledge_dimensions,
    BLOOM_KNOWLEDGE_DIMENSIONS.length,
    "Question Library Bloom Knowledge Dimension",
  );
  for (const value of query.bloom_knowledge_dimensions) {
    parameters.append(
      "bloom_knowledge_dimensions",
      libraryObjectSearchEnum(
        value,
        BLOOM_KNOWLEDGE_DIMENSIONS,
        "Question Library Bloom Knowledge Dimension",
      ),
    );
  }
  boundedLibraryObjectSearchFilterValues(
    query.question_types,
    QUESTION_SEARCH_QUESTION_TYPES.length,
    "Question Library question_types",
  );
  for (const questionType of query.question_types) {
    parameters.append(
      "question_types",
      libraryObjectSearchEnum(
        questionType,
        QUESTION_SEARCH_QUESTION_TYPES,
        "Question Library Question Type",
      ),
    );
  }
  if (query.capabilities.length > LIBRARY_OBJECT_SEARCH_CAPABILITIES.length) {
    throw new Error(
      "Question Library capabilities must contain at most the supported capability count",
    );
  }
  for (const capability of query.capabilities) {
    parameters.append(
      "capabilities",
      libraryObjectSearchEnum(
        capability,
        LIBRARY_OBJECT_SEARCH_CAPABILITIES,
        "Question Library capability",
      ),
    );
  }
  if (query.question_licenses.length > QUESTION_LICENSES.length) {
    throw new Error("Question Library Question Licenses must contain at most the supported count");
  }
  for (const questionLicense of query.question_licenses) {
    parameters.append(
      "question_licenses",
      libraryObjectSearchEnum(
        questionLicense,
        QUESTION_LICENSES,
        "Question Library Question License",
      ),
    );
  }
  const authorship = libraryObjectSearchEnum(
    query.authorship,
    ["any", "authoredByCurrentAccount"],
    "Question Library authorship scope",
  );
  // Keep the current visible source explicit in every cursor-bound request.
  // `any` is a closed scope, not an omitted identity fallback.
  parameters.set("authorship", authorship);
  parameters.set(
    "sort",
    libraryObjectSearchEnum(
      query.sort,
      [
        "titleAscending",
        "titleDescending",
        "kindAscending",
        "kindDescending",
        "disciplineAscending",
        "disciplineDescending",
        "questionTypeAscending",
        "questionTypeDescending",
        "publishedNewest",
      ],
      "Question Library sort",
    ),
  );
  boundedLibraryObjectSearchFilterValues(
    query.excluded_question_ids,
    MAX_QUESTION_SEARCH_EXCLUDED_QUESTION_IDS,
    "Question Library excluded Question IDs",
  );
  for (const questionId of query.excluded_question_ids) {
    const canonicalQuestionId = validateCanonicalQuestionIdSyntax(questionId);
    if (canonicalQuestionId === null) {
      throw new Error("Question Library excluded Question ID must be canonical");
    }
    parameters.append("excluded_question_ids", canonicalQuestionId);
  }
  if (query.page_number !== null) {
    if (
      !Number.isSafeInteger(query.page_number) ||
      query.page_number < 1 ||
      query.page_number > 0xffffffff
    ) {
      throw new Error("Question Library page_number must be an integer from 1 through 4294967295");
    }
    parameters.set("page_number", String(query.page_number));
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
  return parameters;
}

export function libraryObjectSearchPath(query: LibraryObjectSearchRequest): string {
  if (query.cursor !== null) {
    throw new Error("Question Library result requests use numbered pagination");
  }
  return `/api/library-objects/search?${libraryObjectSearchParameters(query).toString()}`;
}

const LIBRARY_FACET_FIELDS: ReadonlyArray<LibraryFacetField> = [
  "kind",
  "authorNames",
  "backends",
  "tags",
  "nameSubjects",
  "nameTopics",
  "disciplineUuids",
  "subjectUuids",
  "topicUuids",
  "subtopicUuids",
  "bloomCognitiveProcesses",
  "bloomKnowledgeDimensions",
  "questionTypes",
  "capabilities",
  "questionLicenses",
];

export function libraryFacetChoicesPath(
  field: LibraryFacetField,
  query: LibraryObjectSearchRequest,
  choiceSearch: string | null = null,
): string {
  if (!LIBRARY_FACET_FIELDS.includes(field)) {
    throw new Error("Question Library facet field is invalid");
  }
  if (query.page_size !== null && query.page_size > 100) {
    throw new Error("Question Library choice page_size must be at most 100");
  }
  const parameters = libraryObjectSearchParameters(query);
  if (query.cursor !== null) {
    if (query.cursor.length === 0 || query.cursor.length > 1024) {
      throw new Error("Question Library facet choice cursor is invalid");
    }
    parameters.set("cursor", query.cursor);
  }
  if (choiceSearch !== null) {
    if (Array.from(choiceSearch).length > 128) {
      throw new Error("Question Library choice search is too long");
    }
    parameters.set("choice_search", choiceSearch);
  }
  return `/api/library-objects/search/facet-choices/${encodeURIComponent(field)}?${parameters.toString()}`;
}

/** Serializes one exact canonical Question ID after local checksum validation. */
export function questionIdPath(questionId: string): string {
  const canonicalQuestionId = validateCanonicalQuestionIdSyntax(questionId);
  if (canonicalQuestionId === null)
    throw new Error("Question ID must use canonical Crockford entry syntax");
  return `/api/questions/by-id/${encodeURIComponent(canonicalQuestionId)}`;
}

/** Serializes one canonical public ID for the authenticated Library kind lookup. */
export function libraryObjectKindPath(publicId: string): string {
  const canonicalPublicId = validateCanonicalQuestionIdSyntax(publicId);
  if (canonicalPublicId === null) {
    throw new Error("Library object ID must use canonical Crockford entry syntax");
  }
  return `/api/library/${encodeURIComponent(canonicalPublicId)}/kind`;
}
