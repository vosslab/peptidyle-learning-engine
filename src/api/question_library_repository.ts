// question_library_repository.ts - converts the generated Question Library search contract for the Library UI.

import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { Capability } from "../../generated/api/Capability";
import type { QuestionLicense } from "../../generated/api/QuestionLicense";
import type { QuestionType } from "../../generated/api/QuestionType";
import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import {
  isBloomCognitiveProcess,
  isBloomKnowledgeDimension,
} from "./decoders/bloom_classification";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import type { ApiClient } from "./client";
import { validateCanonicalQuestionIdSyntax } from "../question_id";
import { libraryClassificationFilter } from "./library_classification_filter";
import { isProductionQuestionBackend, PRODUCTION_QUESTION_BACKENDS } from "./decoders/shared";
import type {
  QuestionLibraryBrowseQuery,
  QuestionLibraryBrowseRepository,
  QuestionLibraryBrowseFacetAggregate,
  QuestionLibraryBrowsePage,
} from "../pages/library_page_model";

const QUESTION_LIBRARY_PAGE_SIZE = 50;
const CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "questionAttemptTimeLimit",
  "printExport",
  "offlinePreview",
] as const satisfies ReadonlyArray<Capability>;
const QUESTION_LICENSES = [
  "CC0-1.0",
  "CC-BY-4.0",
  "CC-BY-SA-4.0",
] as const satisfies ReadonlyArray<QuestionLicense>;
const QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const satisfies ReadonlyArray<QuestionType>;

/**
 * The exact selected Published Question identities a later bulk command may
 * carry. This is intentionally selection-only: it does not name an operation
 * or include mutable metadata before those product boundaries exist.
 */
export interface QuestionLibraryBulkSelectionRequest {
  readonly questionIds: ReadonlyArray<PublishedQuestionId>;
}

/**
 * Closes the browser-side boundary for a nonempty, distinct Question Library
 * selection. The later bulk-operation client owns the route and command.
 */
export function questionLibraryBulkSelectionRequest(
  questionIds: ReadonlyArray<string>,
): QuestionLibraryBulkSelectionRequest {
  if (questionIds.length === 0) {
    throw new Error("A Question Library bulk operation requires at least one Question ID");
  }
  if (questionIds.length > MAX_BULK_QUESTION_METADATA_ITEMS) {
    throw new Error(
      `A Question Library bulk operation accepts at most ${MAX_BULK_QUESTION_METADATA_ITEMS} Questions`,
    );
  }
  const normalized = questionIds.map((questionId) => validateCanonicalQuestionIdSyntax(questionId));
  if (normalized.some((questionId) => questionId === null)) {
    throw new Error("A Question Library bulk operation requires canonical Question IDs");
  }
  const canonicalQuestionIds = normalized as Array<PublishedQuestionId>;
  if (new Set(canonicalQuestionIds).size !== canonicalQuestionIds.length) {
    throw new Error("A Question Library bulk operation cannot select a Question more than once");
  }
  canonicalQuestionIds.sort();
  return { questionIds: canonicalQuestionIds };
}

function selectedCapability(value: string | null): Array<Capability> {
  if (value === null) {
    return [];
  }
  const selected = CAPABILITIES.find((candidate) => candidate === value);
  if (selected === undefined) {
    throw new Error("Question Library capability selection is invalid");
  }
  return [selected];
}

function selectedQuestionLicense(value: string | null): Array<QuestionLicense> {
  if (value === null) {
    return [];
  }
  const selected = QUESTION_LICENSES.find((candidate) => candidate === value);
  if (selected === undefined) {
    throw new Error("Question Library Question License selection is invalid");
  }
  return [selected];
}

function selectedBackend(value: string | null): QuestionSearchRequest["backends"] {
  if (value === null) return [];
  const selected = PRODUCTION_QUESTION_BACKENDS.find((candidate) => candidate === value);
  if (selected === undefined) throw new Error("Question Library backend selection is invalid");
  return [selected];
}

function selectedQuestionType(value: string | null): QuestionSearchRequest["question_types"] {
  if (value === null) return [];
  const selected = QUESTION_TYPES.find((candidate) => candidate === value);
  if (selected === undefined)
    throw new Error("Question Library Question Type selection is invalid");
  return [selected];
}

function selectedPublicText(value: string | null): Array<string> {
  return value === null ? [] : [value];
}

function selectedPublicTexts(values: ReadonlyArray<string>): Array<string> {
  return [...values];
}

function selectedBloomCognitiveProcess(value: string | null): BloomCognitiveProcess | null {
  if (value === null || isBloomCognitiveProcess(value)) return value;
  throw new Error("Question Library Bloom Cognitive Process selection is invalid");
}

function selectedBloomKnowledgeDimension(value: string | null): BloomKnowledgeDimension | null {
  if (value === null || isBloomKnowledgeDimension(value)) return value;
  throw new Error("Question Library Bloom Knowledge Dimension selection is invalid");
}

function facets(
  page: Awaited<ReturnType<ApiClient["searchQuestionLibrary"]>>,
): ReadonlyArray<QuestionLibraryBrowseFacetAggregate> {
  return [
    ...page.facets.authorNames.map((facet) => ({
      facet: "authorName" as const,
      value: facet.authorName,
      count: facet.count,
    })),
    ...page.facets.backends
      .filter((facet) => isProductionQuestionBackend(facet.backend))
      .map((facet) => ({
        facet: "backend" as const,
        value: facet.backend,
        count: facet.count,
      })),
    ...page.facets.tags.map((facet) => ({
      facet: "tag" as const,
      value: facet.tag,
      count: facet.count,
    })),
    ...page.facets.subjects.map((facet) => ({
      facet: "subject" as const,
      value: facet.subject,
      count: facet.count,
    })),
    ...page.facets.topics.map((facet) => ({
      facet: "topic" as const,
      value: facet.topic,
      count: facet.count,
    })),
    ...page.facets.questionTypes.map((facet) => ({
      facet: "questionType" as const,
      value: facet.questionType,
      count: facet.count,
    })),
    ...page.facets.capabilities.map((facet) => ({
      facet: "capability" as const,
      value: facet.capability,
      count: facet.count,
    })),
    ...page.facets.questionLicenses.map((facet) => ({
      facet: "questionLicense" as const,
      value: facet.questionLicense,
      count: facet.count,
    })),
    ...page.facets.bloomCognitiveProcesses.map((facet) => ({
      facet: "bloomCognitiveProcess" as const,
      value: facet.cognitiveProcess,
      count: facet.count,
    })),
    ...page.facets.bloomKnowledgeDimensions.map((facet) => ({
      facet: "bloomKnowledgeDimension" as const,
      value: facet.knowledgeDimension,
      count: facet.count,
    })),
    { facet: "usedInMyCourses" as const, value: "used", count: page.facets.usedInMyCourses.used },
  ];
}

/** Builds the one closed Question Library search request used by Library and source-aware pickers. */
export function questionSearchRequest(
  query: QuestionLibraryBrowseQuery,
  cursor: string | null,
  authorship: QuestionSearchAuthorship = "any",
): QuestionSearchRequest {
  return {
    text: query.search === "" ? null : query.search,
    ...libraryClassificationFilter(query),
    author_names: selectedPublicText(query.authorName),
    backends: selectedBackend(query.backend),
    tags: selectedPublicText(query.tag),
    subjects: selectedPublicTexts(query.subjects),
    topics: selectedPublicTexts(query.topics),
    bloom_cognitive_process: selectedBloomCognitiveProcess(query.bloomCognitiveProcess),
    bloom_knowledge_dimension: selectedBloomKnowledgeDimension(query.bloomKnowledgeDimension),
    question_types: selectedQuestionType(query.questionType),
    capabilities: selectedCapability(query.capability),
    question_licenses: selectedQuestionLicense(query.questionLicense),
    used_in_my_courses: query.usedInMyCourses === "used" ? "used" : "any",
    authorship,
    sort: query.sort,
    cursor,
    page_size: QUESTION_LIBRARY_PAGE_SIZE,
  };
}

/** The only production bridge from the generated client into the virtual Question Library surface. */
export function createQuestionLibraryRepository(
  client: ApiClient,
  authorship: QuestionSearchAuthorship = "any",
): QuestionLibraryBrowseRepository {
  return {
    async search(query: QuestionLibraryBrowseQuery, cursor: string | null): Promise<unknown> {
      const search = questionSearchRequest(query, cursor, authorship);
      const page = await client.searchQuestionLibrary(search);
      return {
        items: page.items.map((item) => ({
          displayId: item.summary.questionId,
          publishedQuestionRevisionTuple: item.summary.publishedQuestionRevisionTuple,
          questionTitle: item.summary.metadata.questionTitle,
          summary: item.summary.metadata.questionDescription,
          bloom: item.summary.bloom,
          disciplineName: item.disciplineName,
          disciplineIsRetired: item.disciplineIsRetired,
          questionFormat: item.summary.questionFormat,
          authorNames: item.summary.authorship.authors.map((author) => author.displayName),
          capabilities: item.summary.capabilities,
          questionLicense: item.summary.metadata.questionLicense,
          evidence: item.evidence,
        })),
        nextCursor: page.nextCursor,
        aggregates: facets(page),
        facetTruncation: {
          authorNames: page.facets.authorNamesTruncated,
          tags: page.facets.tagsTruncated,
          subjects: page.facets.subjectsTruncated,
          topics: page.facets.topicsTruncated,
        },
      } satisfies QuestionLibraryBrowsePage;
    },
  };
}
