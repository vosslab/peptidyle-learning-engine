// library_page_model.ts - bounded, transport-validated Question Library browse state.

import type { SearchState } from "../features/search/search_session";
import type { RecordPageSize } from "../components/record_list/record_page_sizes";
import { validateCanonicalQuestionIdSyntax } from "../question_id";
import {
  EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  libraryClassificationFilter,
  type LibraryClassificationFilter,
} from "../api/library_classification_filter";
import type { QuestionFormat } from "../../generated/api/QuestionFormat";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";
import type { QuestionSearchSort } from "../../generated/api/QuestionSearchSort";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionAuthor } from "../../generated/api/QuestionAuthor";
import type { QuestionPoolMetadataEditNumber } from "../../generated/api/QuestionPoolMetadataEditNumber";
import type { QuestionPoolEditNumber } from "../../generated/api/QuestionPoolEditNumber";
import { decodeQuestionAuthorship } from "../api/question_authorship";
import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "../api/decoders/bloom_classification";
import { MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES } from "../../generated/api/MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES";
import { MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS } from "../../generated/api/MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS";
import { MAX_QUESTION_SEARCH_TAG_FACETS } from "../../generated/api/MAX_QUESTION_SEARCH_TAG_FACETS";
import { MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS } from "../api/decoders/question_type_facets";
import {
  MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
  MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS,
  QUESTION_BACKENDS,
  decodePublishedQuestionRevisionTuple,
} from "../api/decoders/shared";
import { decodeQuestionStatistics } from "../api/decoders/question_library";
import { decodeBloomClassificationView } from "../api/decoders/bloom_classification";

/** A browser-safe current Question Library record. */
export interface QuestionLibraryBrowseRow {
  readonly kind: "question";
  /** Copy/paste identity used by instructors and the browser deduplication key. */
  readonly displayId: string;
  /** Exact immutable revision selected by this browse result. */
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly summary: string;
  /** Exact Revision-owned Bloom pair and its independent correction precondition, when assigned. */
  readonly bloom: BloomClassificationView | null;
  /** Current readable Discipline name for this Question's existing classification. */
  readonly disciplineName: string;
  /** Existing references may retain a retired Discipline. */
  readonly disciplineIsRetired: boolean;
  /** Immutable source representation, without source location or content.
   * Retained Assessment picker candidates have no format projection, so they
   * explicitly retain unavailable metadata rather than guessing from backend. */
  readonly questionFormat: QuestionFormat | null;
  /** Immutable reviewed authorship, optionally linked to a real active Instructor Profile. */
  readonly authors: ReadonlyArray<QuestionAuthor>;
  readonly capabilities: ReadonlyArray<string>;
  readonly questionLicense: string | null;
  /** Server-disclosed learning evidence for this exact immutable publication. */
  readonly evidence: QuestionStatistics;
}

/** A Pool row carries only Pool-owned facts; Question source fields never leak into this arm. */
export interface QuestionLibraryPoolRow {
  readonly kind: "pool";
  readonly displayId: string;
  readonly title: string;
  readonly description: string;
  readonly ownerAccountId: string;
  readonly memberCount: number;
  readonly questionType: string;
  readonly backend: string;
  readonly license: string;
  readonly disciplineName: string;
  readonly disciplineIsRetired: boolean;
  readonly tags: ReadonlyArray<string>;
  readonly bloom: BloomClassificationView | null;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
  /** Current Pool membership/edit concurrency token for an import, distinct from metadata edits. */
  readonly questionPoolEditNumber: QuestionPoolEditNumber;
  readonly subjectUuid: string;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
}

/** The Library alone admits both kinds. Pickers continue to use QuestionLibraryBrowseRow. */
export type LibrarySearchRow = QuestionLibraryBrowseRow | QuestionLibraryPoolRow;

/**
 * A presentation-ready, answer-free view of the server-owned discovery evidence.
 */
export type QuestionLibraryBrowseEvidence = QuestionStatistics;

/** Server-computed count for the exact active query; never derived from loaded rows. */
export interface QuestionLibraryBrowseFacetAggregate {
  readonly facet:
    | "authorName"
    | "backend"
    | "tag"
    | "subject"
    | "topic"
    | "questionType"
    | "capability"
    | "questionLicense"
    | "usedInMyCourses"
    | "bloomCognitiveProcess"
    | "bloomKnowledgeDimension";
  readonly value: string;
  readonly count: number;
}

export interface QuestionLibraryBrowseQuery extends LibraryClassificationFilter {
  readonly kind: "both" | "questions" | "pools";
  readonly membership: "noPool" | "all";
  readonly ownerAccountId: string | null;
  readonly search: string;
  readonly authorName: string | null;
  readonly backend: string | null;
  readonly tag: string | null;
  readonly subjects: ReadonlyArray<string>;
  readonly topics: ReadonlyArray<string>;
  readonly bloomCognitiveProcess: BloomCognitiveProcess | null;
  readonly bloomKnowledgeDimension: BloomKnowledgeDimension | null;
  readonly questionType: string | null;
  readonly capability: string | null;
  readonly questionLicense: string | null;
  readonly usedInMyCourses: string | null;
  /** Closed server-resolved authorship scope; browser rows never carry Account identity. */
  readonly authorship: QuestionSearchAuthorship;
  /** Server-owned deterministic order retained with this exact query. */
  readonly sort: QuestionSearchSort;
}

/** Honest notice that a free-form facet group is only the bounded leading set. */
export interface QuestionLibraryFacetTruncation {
  readonly authorNames: boolean;
  readonly tags: boolean;
  readonly subjects: boolean;
  readonly topics: boolean;
}

export const NO_QUESTION_LIBRARY_FACET_TRUNCATION: QuestionLibraryFacetTruncation = {
  authorNames: false,
  tags: false,
  subjects: false,
  topics: false,
};

export interface QuestionLibraryBrowsePage {
  readonly items: ReadonlyArray<QuestionLibraryBrowseRow>;
  readonly nextCursor: string | null;
  readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
  readonly facetTruncation: QuestionLibraryFacetTruncation;
}

export interface LibrarySearchPage extends Omit<QuestionLibraryBrowsePage, "items"> {
  readonly items: ReadonlyArray<LibrarySearchRow>;
}

/** Shared discovery choices. The repository sends one of these sizes on each search. */
export type QuestionLibraryPageSize = RecordPageSize;

/** Cursor sequence for the current query and page size. The first page uses a null input cursor. */
export interface QuestionLibraryBrowsePosition {
  readonly pageSize: QuestionLibraryPageSize;
  readonly inputCursor: string | null;
  readonly previousCursors: ReadonlyArray<string | null>;
}

export const FIRST_QUESTION_LIBRARY_BROWSE_POSITION: QuestionLibraryBrowsePosition = {
  pageSize: 50,
  inputCursor: null,
  previousCursors: [],
};

/**
 * The production Question Library repository adapts to this narrow boundary. A hostile result is
 * intentional: this module owns browser-side validation before any row reaches JSX.
 */
export interface QuestionLibraryBrowseRepository {
  readonly search: (
    query: QuestionLibraryBrowseQuery,
    cursor: string | null,
    pageSize?: QuestionLibraryPageSize,
  ) => Promise<unknown>;
}

export interface QuestionLibraryFilterCounts {
  readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
  readonly facetTruncation: QuestionLibraryFacetTruncation;
}

export type QuestionLibraryBrowseState = SearchState<
  QuestionLibraryBrowseRow,
  QuestionLibraryFilterCounts
>;
export type LibrarySearchState = SearchState<LibrarySearchRow, QuestionLibraryFilterCounts>;

const MAX_TEXT_LENGTH = 512;
const MAX_SUMMARY_LENGTH = 4_000;
export const MAX_QUESTION_LIBRARY_BROWSE_PAGE_ITEMS = 250;
const MAX_QUESTION_LIBRARY_ROW_TEXT_ITEMS = 100;
const MAX_FACET_COUNT = 1_000_000_000;

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Readonly<Record<string, unknown>>,
  keys: ReadonlyArray<string>,
): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

function boundedText(value: unknown, path: string, maxLength = MAX_TEXT_LENGTH): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maxLength) {
    throw new Error(`${path} must be non-empty text within ${maxLength} characters`);
  }
  return value;
}

function decodeNullableQuestionLicense(value: unknown, path: string): string | null {
  if (value === null) return null;
  if (value !== "CC0-1.0" && value !== "CC-BY-4.0" && value !== "CC-BY-SA-4.0") {
    throw new Error(`${path} must be an exact Question License or null`);
  }
  return value;
}

function decodeQuestionFormat(value: unknown, path: string): QuestionFormat {
  if (
    value !== "pleQuestionJson" &&
    value !== "webworkPg" &&
    value !== "webworkPgml" &&
    value !== "imathas"
  ) {
    throw new Error(`${path} must be an exact published Question Format`);
  }
  return value;
}

function stringList(value: unknown, path: string): ReadonlyArray<string> {
  if (!Array.isArray(value) || value.length > MAX_QUESTION_LIBRARY_ROW_TEXT_ITEMS) {
    throw new Error(`${path} must be an array`);
  }
  return value.map((item, index) => boundedText(item, `${path}[${index}]`));
}

function decodeRow(value: unknown, path: string): QuestionLibraryBrowseRow {
  const authorKey = isRecord(value) && "authors" in value ? "authors" : "authorNames";
  const kindKey = isRecord(value) && "kind" in value ? ["kind"] : [];
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      authorKey,
      "capabilities",
      "displayId",
      "questionLicense",
      "questionFormat",
      "publishedQuestionRevisionTuple",
      "summary",
      "bloom",
      "questionTitle",
      "disciplineName",
      "disciplineIsRetired",
      "evidence",
      ...kindKey,
    ])
  ) {
    throw new Error(`${path} has an unexpected shape`);
  }
  if ("kind" in value && value["kind"] !== "question") {
    throw new Error(`${path}.kind must be question`);
  }
  const rawDisplayId = boundedText(value["displayId"], `${path}.displayId`);
  const displayId = validateCanonicalQuestionIdSyntax(rawDisplayId);
  if (displayId === null || displayId !== rawDisplayId) {
    throw new Error(`${path}.displayId must be a canonical Question ID`);
  }
  const evidence = decodeBrowseEvidence(value["evidence"], `${path}.evidence`);
  const disciplineIsRetired = value["disciplineIsRetired"];
  if (typeof disciplineIsRetired !== "boolean") {
    throw new Error(`${path}.disciplineIsRetired must be boolean`);
  }
  const publishedQuestionRevisionTuple = decodePublishedQuestionRevisionTuple(
    value["publishedQuestionRevisionTuple"],
    `${path}.publishedQuestionRevisionTuple`,
    true,
  );
  if (publishedQuestionRevisionTuple.publishedQuestionId !== displayId) {
    throw new Error(
      `${path}.publishedQuestionRevisionTuple.publishedQuestionId must match displayId`,
    );
  }
  return {
    kind: "question",
    displayId,
    publishedQuestionRevisionTuple,
    questionTitle: boundedText(value["questionTitle"], `${path}.questionTitle`),
    summary: boundedText(value["summary"], `${path}.summary`, MAX_SUMMARY_LENGTH),
    bloom:
      value["bloom"] === null
        ? null
        : decodeBloomClassificationView(value["bloom"], `${path}.bloom`),
    disciplineName: boundedText(value["disciplineName"], `${path}.disciplineName`, 120),
    disciplineIsRetired,
    questionFormat: decodeQuestionFormat(value["questionFormat"], `${path}.questionFormat`),
    authors:
      authorKey === "authors"
        ? decodeQuestionAuthorship({ authors: value["authors"] }, `${path}.authors`).authors
        : stringList(value["authorNames"], `${path}.authorNames`).map((displayName) => ({
            displayName,
            accountId: null,
          })),
    capabilities: stringList(value["capabilities"], `${path}.capabilities`),
    questionLicense: decodeNullableQuestionLicense(
      value["questionLicense"],
      `${path}.questionLicense`,
    ),
    evidence,
  };
}

function decodePoolRow(value: unknown, path: string): QuestionLibraryPoolRow {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "backend",
      "bloom",
      "description",
      "disciplineIsRetired",
      "disciplineName",
      "displayId",
      "kind",
      "license",
      "memberCount",
      "ownerAccountId",
      "questionPoolMetadataEditNumber",
      "questionPoolEditNumber",
      "questionType",
      "subjectUuid",
      "subtopicUuid",
      "tags",
      "title",
      "topicUuid",
    ])
  ) {
    throw new Error(`${path} has an unexpected Pool row shape`);
  }
  if (value["kind"] !== "pool") throw new Error(`${path}.kind must be pool`);
  const memberCount = value["memberCount"];
  const editNumber = value["questionPoolMetadataEditNumber"];
  const questionPoolEditNumber = value["questionPoolEditNumber"];
  if (typeof memberCount !== "number" || !Number.isSafeInteger(memberCount) || memberCount < 0) {
    throw new Error(`${path}.memberCount must be a non-negative safe integer`);
  }
  if (typeof editNumber !== "number" || !Number.isSafeInteger(editNumber) || editNumber < 1) {
    throw new Error(`${path}.questionPoolMetadataEditNumber must be a positive integer`);
  }
  if (
    typeof questionPoolEditNumber !== "number" ||
    !Number.isSafeInteger(questionPoolEditNumber) ||
    questionPoolEditNumber < 1
  ) {
    throw new Error(`${path}.questionPoolEditNumber must be a positive integer`);
  }
  return {
    kind: "pool",
    displayId: boundedText(value["displayId"], `${path}.displayId`),
    title: boundedText(value["title"], `${path}.title`),
    description: boundedText(value["description"], `${path}.description`, MAX_SUMMARY_LENGTH),
    ownerAccountId: boundedText(value["ownerAccountId"], `${path}.ownerAccountId`),
    memberCount,
    questionType: boundedText(value["questionType"], `${path}.questionType`),
    backend: boundedText(value["backend"], `${path}.backend`),
    license: boundedText(value["license"], `${path}.license`),
    disciplineName: boundedText(value["disciplineName"], `${path}.disciplineName`),
    disciplineIsRetired:
      typeof value["disciplineIsRetired"] === "boolean"
        ? value["disciplineIsRetired"]
        : ((): never => {
            throw new Error(`${path}.disciplineIsRetired must be boolean`);
          })(),
    tags: stringList(value["tags"], `${path}.tags`),
    bloom:
      value["bloom"] === null
        ? null
        : decodeBloomClassificationView(value["bloom"], `${path}.bloom`),
    questionPoolMetadataEditNumber: editNumber,
    questionPoolEditNumber,
    subjectUuid: boundedText(value["subjectUuid"], `${path}.subjectUuid`),
    topicUuid:
      value["topicUuid"] === null ? null : boundedText(value["topicUuid"], `${path}.topicUuid`),
    subtopicUuid:
      value["subtopicUuid"] === null
        ? null
        : boundedText(value["subtopicUuid"], `${path}.subtopicUuid`),
  };
}

function decodeBrowseEvidence(value: unknown, path: string): QuestionLibraryBrowseEvidence {
  return decodeQuestionStatistics(value, path);
}

function decodeAggregate(value: unknown, path: string): QuestionLibraryBrowseFacetAggregate {
  if (!isRecord(value) || !hasExactKeys(value, ["count", "facet", "value"])) {
    throw new Error(`${path} has an unexpected shape`);
  }
  const facet = value["facet"];
  if (
    facet !== "authorName" &&
    facet !== "backend" &&
    facet !== "tag" &&
    facet !== "subject" &&
    facet !== "topic" &&
    facet !== "questionType" &&
    facet !== "capability" &&
    facet !== "questionLicense" &&
    facet !== "usedInMyCourses" &&
    facet !== "bloomCognitiveProcess" &&
    facet !== "bloomKnowledgeDimension"
  ) {
    throw new Error(`${path}.facet is not a Question Library facet`);
  }
  const count = value["count"];
  if (
    typeof count !== "number" ||
    !Number.isSafeInteger(count) ||
    count < 0 ||
    count > MAX_FACET_COUNT
  ) {
    throw new Error(`${path}.count must be a non-negative safe integer`);
  }
  const aggregateValue = boundedText(value["value"], `${path}.value`);
  if (
    (facet === "bloomCognitiveProcess" &&
      !BLOOM_COGNITIVE_PROCESSES.some((candidate) => candidate === aggregateValue)) ||
    (facet === "bloomKnowledgeDimension" &&
      !BLOOM_KNOWLEDGE_DIMENSIONS.some((candidate) => candidate === aggregateValue))
  ) {
    throw new Error(`${path}.value is not an exact Bloom classification value`);
  }
  return { facet, value: aggregateValue, count };
}

function decodeFacetTruncation(value: unknown): QuestionLibraryFacetTruncation {
  if (!isRecord(value) || !hasExactKeys(value, ["authorNames", "tags", "subjects", "topics"])) {
    throw new Error("Question Library facet truncation has an unexpected shape");
  }
  const authorNames = value["authorNames"];
  const tags = value["tags"];
  const subjects = value["subjects"];
  const topics = value["topics"];
  if (
    typeof authorNames !== "boolean" ||
    typeof tags !== "boolean" ||
    typeof subjects !== "boolean" ||
    typeof topics !== "boolean"
  ) {
    throw new Error("Question Library facet truncation fields must be boolean");
  }
  return {
    authorNames,
    tags,
    subjects,
    topics,
  };
}

function validateAggregateGroupCaps(
  aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>,
): void {
  const caps: Readonly<Record<QuestionLibraryBrowseFacetAggregate["facet"], number>> = {
    authorName: MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS,
    backend: QUESTION_BACKENDS.length,
    tag: MAX_QUESTION_SEARCH_TAG_FACETS,
    subject: MAX_QUESTION_SEARCH_TAG_FACETS,
    topic: MAX_QUESTION_SEARCH_TAG_FACETS,
    questionType: MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
    capability: MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
    questionLicense: MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS,
    usedInMyCourses: 1,
    bloomCognitiveProcess: BLOOM_COGNITIVE_PROCESSES.length,
    bloomKnowledgeDimension: BLOOM_KNOWLEDGE_DIMENSIONS.length,
  };
  const counts = new Map<QuestionLibraryBrowseFacetAggregate["facet"], number>();
  for (const aggregate of aggregates) {
    const next = (counts.get(aggregate.facet) ?? 0) + 1;
    if (next > caps[aggregate.facet]) {
      throw new Error(`Question Library ${aggregate.facet} aggregate group is too large`);
    }
    counts.set(aggregate.facet, next);
  }
}

/** Strictly decode the live/generated-client result before browser presentation. */
export function decodeQuestionLibraryBrowsePage(value: unknown): QuestionLibraryBrowsePage {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["aggregates", "facetTruncation", "items", "nextCursor"])
  ) {
    throw new Error("Question Library response has an unexpected shape");
  }
  if (
    !Array.isArray(value["items"]) ||
    value["items"].length > MAX_QUESTION_LIBRARY_BROWSE_PAGE_ITEMS ||
    !Array.isArray(value["aggregates"])
  ) {
    throw new Error("Question Library response arrays are invalid");
  }
  const nextCursor = value["nextCursor"];
  if (
    nextCursor !== null &&
    (typeof nextCursor !== "string" ||
      nextCursor.length === 0 ||
      nextCursor.length > MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES)
  ) {
    throw new Error("Question Library response cursor is invalid");
  }
  const aggregates = value["aggregates"].map((item, index) =>
    decodeAggregate(item, `aggregates[${index}]`),
  );
  validateAggregateGroupCaps(aggregates);
  return {
    items: value["items"].map((item, index) => decodeRow(item, `items[${index}]`)),
    nextCursor,
    aggregates,
    facetTruncation: decodeFacetTruncation(value["facetTruncation"]),
  };
}

/** Strictly decodes the mixed Library page while retaining the Question-only picker decoder. */
export function decodeLibrarySearchPage(value: unknown): LibrarySearchPage {
  const page = decodeQuestionLibraryBrowsePage({
    ...(isRecord(value) ? value : {}),
    items: [],
  });
  if (!isRecord(value) || !Array.isArray(value["items"])) {
    throw new Error("Question Library response arrays are invalid");
  }
  if (value["items"].length > MAX_QUESTION_LIBRARY_BROWSE_PAGE_ITEMS) {
    throw new Error("Question Library response arrays are invalid");
  }
  return {
    ...page,
    items: value["items"].map((item, index) => {
      if (!isRecord(item)) throw new Error(`items[${index}] has an unexpected shape`);
      if (item["kind"] === "pool") return decodePoolRow(item, `items[${index}]`);
      return decodeRow(item, `items[${index}]`);
    }),
  };
}

export const EMPTY_QUESTION_LIBRARY_BROWSE_QUERY: QuestionLibraryBrowseQuery = {
  ...EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  kind: "both",
  membership: "noPool",
  ownerAccountId: null,
  search: "",
  authorName: null,
  backend: null,
  tag: null,
  subjects: [],
  topics: [],
  bloomCognitiveProcess: null,
  bloomKnowledgeDimension: null,
  questionType: null,
  capability: null,
  questionLicense: null,
  usedInMyCourses: null,
  authorship: "any",
  sort: "titleAscending",
};

export function normalizeQuestionLibraryBrowseQuery(
  query: QuestionLibraryBrowseQuery,
): QuestionLibraryBrowseQuery {
  const poolsOnly = query.kind === "pools";
  return {
    ...libraryClassificationFilter(query),
    kind: query.kind,
    membership: poolsOnly ? "all" : query.membership,
    ownerAccountId: query.ownerAccountId === null ? null : query.ownerAccountId.trim(),
    search: query.search.trim().replace(/\s+/g, " "),
    authorName: poolsOnly ? null : query.authorName,
    backend: query.backend,
    tag: query.tag,
    subjects: query.subjects.map((subject) => subject.trim().replace(/\s+/g, " ")),
    topics: query.topics.map((topic) => topic.trim().replace(/\s+/g, " ")),
    bloomCognitiveProcess: query.bloomCognitiveProcess,
    bloomKnowledgeDimension: query.bloomKnowledgeDimension,
    questionType: query.questionType,
    capability: poolsOnly ? null : query.capability,
    questionLicense: query.questionLicense,
    usedInMyCourses: poolsOnly ? null : query.usedInMyCourses,
    authorship: poolsOnly ? "any" : query.authorship,
    sort: query.sort,
  };
}
