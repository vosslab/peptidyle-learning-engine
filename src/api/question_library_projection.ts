// Shared browser projection for strictly decoded Library Object results.

import type { SearchPageRequest } from "../components/search_block/search_definition";
import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import type { RecordFilterChoices } from "../components/data_sheet/data_sheet_fields";
import type { RecordPageSize } from "../components/data_sheet/data_sheet_page_sizes";
import type { LibraryClassificationFilter } from "./library_classification_filter";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { QuestionFormat } from "../../generated/api/QuestionFormat";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";
import type { LibraryObjectSearchSort } from "../../generated/api/LibraryObjectSearchSort";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionAuthor } from "../../generated/api/QuestionAuthor";
import type { QuestionPoolMetadataEditNumber } from "../../generated/api/QuestionPoolMetadataEditNumber";
import type { QuestionPoolEditNumber } from "../../generated/api/QuestionPoolEditNumber";
import type { LibrarySearchClassification } from "../../generated/api/LibrarySearchClassification";
import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";

/** A browser-safe current Question Library record. */
export interface QuestionLibraryBrowseRow {
  readonly kind: "question";
  /** Copy/paste identity used by instructors and the browser deduplication key. */
  readonly displayId: string;
  /** Exact immutable revision selected by this browse result. */
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  /** Registered thumbnail relation for this exact revision, when present. */
  readonly questionThumbnail: PublishedQuestionRevisionTuple | null;
  readonly nativePrompt: ReadonlyArray<QuestionContentBlock> | null;
  readonly questionTitle: string;
  readonly summary: string;
  /** Display-only Bloom projection for the exact Question Revision, when assigned. */
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

/** Shared fields projected from each Library Object kind at the browser row boundary. */
export interface LibraryObjectCommonRow {
  readonly classification: LibrarySearchClassification;
  readonly title: string;
  readonly description: string;
  readonly ownerAccountId: string;
  readonly questionType: string;
  readonly backend: string;
  readonly tags: ReadonlyArray<string>;
  readonly license: string | null;
}

/** A mixed-search Question row keeps its Question-only revision and evidence fields. */
export interface LibraryObjectQuestionRow
  extends QuestionLibraryBrowseRow, LibraryObjectCommonRow {}

/** A Pool row carries only Pool-owned facts; Question source fields never leak into this arm. */
export interface QuestionLibraryPoolRow extends LibraryObjectCommonRow {
  readonly kind: "pool";
  readonly displayId: string;
  readonly memberCount: number;
  readonly license: string;
  readonly disciplineName: string;
  readonly disciplineIsRetired: boolean;
  readonly bloom: BloomClassificationView | null;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
  /** Current Pool membership/edit concurrency token for an import, distinct from metadata edits. */
  readonly questionPoolEditNumber: QuestionPoolEditNumber;
  readonly subjectUuid: string;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
}

/** Answer-free Question and Pool rows for Library and Assessment discovery. */
export type LibrarySearchRow = LibraryObjectQuestionRow | QuestionLibraryPoolRow;

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
    | "bloomCognitiveProcess"
    | "bloomKnowledgeDimension";
  readonly value: string;
  readonly count: number;
}

export interface QuestionLibraryBrowseQuery extends LibraryClassificationFilter {
  readonly kind: "both" | "questions" | "pools";
  readonly questions: "inNoPool" | "all";
  readonly ownerAccountId: string | null;
  readonly search: string;
  readonly authorNames: ReadonlyArray<string>;
  readonly backends: ReadonlyArray<string>;
  readonly tags: ReadonlyArray<string>;
  readonly subjects: ReadonlyArray<string>;
  readonly topics: ReadonlyArray<string>;
  readonly bloomCognitiveProcesses: ReadonlyArray<BloomCognitiveProcess>;
  readonly bloomKnowledgeDimensions: ReadonlyArray<BloomKnowledgeDimension>;
  readonly questionTypes: ReadonlyArray<string>;
  readonly capabilities: ReadonlyArray<string>;
  readonly questionLicenses: ReadonlyArray<string>;
  /** Closed server-resolved authorship scope; browser rows never carry Account identity. */
  readonly authorship: QuestionSearchAuthorship;
  /** Server-owned deterministic order retained with this exact query. */
  readonly sort: LibraryObjectSearchSort;
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
  readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
  readonly facetTruncation: QuestionLibraryFacetTruncation;
}

export interface LibrarySearchPage extends Omit<QuestionLibraryBrowsePage, "items"> {
  readonly items: ReadonlyArray<LibrarySearchRow>;
  readonly pageNumber: number;
  readonly pageSize: QuestionLibraryPageSize;
  readonly totalCount: number;
}

/** Shared discovery choices. The repository sends one of these sizes on each search. */
export type QuestionLibraryPageSize = RecordPageSize;

/** Optional query eligibility for callers that need to exclude exact Questions before paging. */
export interface QuestionLibrarySearchEligibility {
  readonly excludedQuestionIds?: ReadonlyArray<PublishedQuestionId>;
}

/**
 * Typed browser rows projected from a strictly decoded HTTP response.
 */
export interface QuestionLibraryBrowseRepository {
  readonly search: (
    query: QuestionLibraryBrowseQuery,
    page: SearchPageRequest,
    eligibility?: QuestionLibrarySearchEligibility,
  ) => Promise<LibrarySearchPage>;
  readonly listFacetChoices: (
    field: LibraryFacetField,
    query: QuestionLibraryBrowseQuery,
    search: string,
    cursor: string | null,
  ) => Promise<RecordFilterChoices>;
}

export interface QuestionLibraryFilterCounts {
  readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
  readonly facetTruncation: QuestionLibraryFacetTruncation;
}
