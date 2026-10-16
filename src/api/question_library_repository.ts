// question_library_repository.ts - maps mixed Library Object search results into browser rows.

import type { LibraryObjectSearchRequest } from "../../generated/api/LibraryObjectSearchRequest";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { Capability } from "../../generated/api/Capability";
import type { QuestionLicense } from "../../generated/api/QuestionLicense";
import type { QuestionType } from "../../generated/api/QuestionType";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "./decoders/bloom_classification";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import type { ApiClient } from "./client";
import type { RecordFilterChoices } from "../components/record_list/record_fields";
import { RECORD_PAGE_SIZES } from "../components/record_list/record_page_sizes";
import { normalizeHumanEnteredQuestionId, validateCanonicalQuestionIdSyntax } from "../question_id";
import { libraryClassificationFilter } from "./library_classification_filter";
import { isProductionQuestionBackend, PRODUCTION_QUESTION_BACKENDS } from "./decoders/shared";
import {
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowseFacetAggregate,
  type LibrarySearchPage,
  type QuestionLibraryPageSize,
} from "../pages/library_page_model";

function questionLibraryPageSize(
  pageSize: QuestionLibraryPageSize | undefined,
): QuestionLibraryPageSize {
  if (pageSize === undefined) return 50;
  if (RECORD_PAGE_SIZES.includes(pageSize)) return pageSize;
  throw new Error("Question Library page size must be 50, 100, or 250");
}
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

function selectedValues<T extends string>(
  values: ReadonlyArray<string>,
  allowed: ReadonlyArray<T>,
  name: string,
): Array<T> {
  return values.map((value) => {
    const selected = allowed.find((candidate) => candidate === value);
    if (selected === undefined) throw new Error(`Question Library ${name} selection is invalid`);
    return selected;
  });
}

export function libraryFacetAggregates(
  page: Pick<Awaited<ReturnType<ApiClient["searchLibraryObjects"]>>, "facets">,
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
  ];
}

/** Builds the one closed Library Object search request used by Library and source-aware pickers. */
export function libraryObjectSearchRequest(
  query: QuestionLibraryBrowseQuery,
  cursor: string | null,
  authorship: QuestionSearchAuthorship = "any",
  pageSize?: QuestionLibraryPageSize,
): LibraryObjectSearchRequest {
  return {
    kind: query.kind,
    questions: query.questions,
    owner_account_id: query.ownerAccountId,
    text:
      query.search === "" ? null : (normalizeHumanEnteredQuestionId(query.search) ?? query.search),
    ...libraryClassificationFilter(query),
    author_names: [...query.authorNames],
    backends: selectedValues(query.backends, PRODUCTION_QUESTION_BACKENDS, "backend"),
    tags: [...query.tags],
    subjects: [...query.subjects],
    topics: [...query.topics],
    bloom_cognitive_processes: selectedValues(
      query.bloomCognitiveProcesses,
      BLOOM_COGNITIVE_PROCESSES,
      "Bloom Cognitive Process",
    ),
    bloom_knowledge_dimensions: selectedValues(
      query.bloomKnowledgeDimensions,
      BLOOM_KNOWLEDGE_DIMENSIONS,
      "Bloom Knowledge Dimension",
    ),
    question_types: selectedValues(query.questionTypes, QUESTION_TYPES, "Question Type"),
    capabilities: selectedValues(query.capabilities, CAPABILITIES, "capability"),
    question_licenses: selectedValues(
      query.questionLicenses,
      QUESTION_LICENSES,
      "Question License",
    ),
    authorship,
    sort: query.sort,
    cursor,
    page_size: questionLibraryPageSize(pageSize),
  };
}

/** Maps the shared browser-safe Question projection into RecordList fields. */
export function questionLibraryRowFromResult(
  item: Extract<
    Awaited<ReturnType<ApiClient["searchLibraryObjects"]>>["items"][number],
    { kind: "question" }
  >,
): import("../pages/library_page_model").LibraryObjectQuestionRow {
  const question = item.question;
  return {
    kind: "question" as const,
    displayId: question.summary.questionId,
    title: question.summary.metadata.questionTitle,
    description: question.summary.metadata.questionDescription,
    ownerAccountId: item.ownerAccountId,
    questionType: question.summary.questionType,
    backend: question.summary.backend,
    tags: question.summary.metadata.tags,
    license: question.summary.metadata.questionLicense,
    publishedQuestionRevisionTuple: question.summary.publishedQuestionRevisionTuple,
    questionTitle: question.summary.metadata.questionTitle,
    summary: question.summary.metadata.questionDescription,
    bloom: question.summary.bloom,
    disciplineName: question.disciplineName,
    disciplineIsRetired: question.disciplineIsRetired,
    questionFormat: question.summary.questionFormat,
    authors: question.summary.authorship.authors,
    capabilities: question.summary.capabilities,
    questionLicense: question.summary.metadata.questionLicense,
    evidence: question.evidence,
    classification: item.classification,
  };
}

/** The only production bridge from the generated client into the virtual Question Library surface. */
export function createQuestionLibraryRepository(
  client: ApiClient,
  authorship: QuestionSearchAuthorship = "any",
): QuestionLibraryBrowseRepository {
  return {
    async listFacetChoices(field, query, choiceSearch, cursor): Promise<RecordFilterChoices> {
      const request = libraryObjectSearchRequest(query, cursor, authorship, 50);
      const page = await client.listLibraryFacetChoices(field, request, choiceSearch);
      return page;
    },
    async search(
      query: QuestionLibraryBrowseQuery,
      cursor: string | null,
      pageSize?: QuestionLibraryPageSize,
    ): Promise<unknown> {
      const search = libraryObjectSearchRequest(query, cursor, authorship, pageSize);
      const page = await client.searchLibraryObjects(search);
      return {
        items: page.items.map((item) => {
          if (item.kind === "pool") {
            const pool = item.pool;
            return {
              kind: "pool" as const,
              displayId: pool.questionPoolId,
              title: pool.metadata.title,
              description: pool.metadata.description,
              ownerAccountId: pool.ownerAccountId,
              memberCount: pool.memberCount,
              questionType: pool.questionType,
              backend: pool.backend,
              license: pool.license,
              disciplineName: pool.metadata.disciplineName,
              disciplineIsRetired: pool.metadata.disciplineIsRetired,
              tags: pool.metadata.tags,
              bloom: pool.bloom,
              questionPoolMetadataEditNumber: pool.questionPoolMetadataEditNumber,
              questionPoolEditNumber: pool.questionPoolEditNumber,
              subjectUuid: pool.metadata.subjectUuid,
              topicUuid: pool.metadata.topicUuid,
              subtopicUuid: pool.metadata.subtopicUuid,
              classification: item.classification,
            };
          }
          return questionLibraryRowFromResult(item);
        }),
        nextCursor: page.nextCursor,
        aggregates: libraryFacetAggregates(page),
        facetTruncation: {
          authorNames: page.facets.authorNamesTruncated,
          tags: page.facets.tagsTruncated,
          subjects: page.facets.subjectsTruncated,
          topics: page.facets.topicsTruncated,
        },
      } satisfies LibrarySearchPage;
    },
  };
}
