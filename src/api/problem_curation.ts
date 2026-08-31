// Browser-owned command shapes for the server-owned D2 curation aggregate.

import type { CatalogSearchFilter } from "../../generated/api/CatalogSearchFilter";
import type { QuestionCollectionMemberView } from "../../generated/api/QuestionCollectionMemberView";
import type { QuestionCollectionReference } from "../../generated/api/QuestionCollectionReference";
import type { QuestionCollectionSummaryView } from "../../generated/api/QuestionCollectionSummaryView";
import type { SavedProblemSearchReference } from "../../generated/api/SavedProblemSearchReference";
import type { SavedProblemSearchView } from "../../generated/api/SavedProblemSearchView";
import type { CursorPage } from "./contracts";

/** Strong server ETag that callers retain unchanged for a subsequent mutation. */
export type ProblemCurationEtag = string;

/** One current collection projection paired with its authoritative edit-number ETag. */
export interface RevisionedQuestionCollection {
  readonly collection: QuestionCollectionSummaryView;
  readonly etag: ProblemCurationEtag;
}

/** One current saved-search projection paired with its authoritative edit-number ETag. */
export interface RevisionedSavedProblemSearch {
  readonly search: SavedProblemSearchView;
  readonly etag: ProblemCurationEtag;
}

/** A bounded members page tied to the collection edit number returned in its ETag. */
export interface QuestionCollectionMembersPage {
  readonly page: CursorPage<QuestionCollectionMemberView>;
  readonly etag: ProblemCurationEtag;
}

/** Complete collection state submitted atomically; member order is meaningful. */
export interface QuestionCollectionReplaceRequest {
  readonly title?: string;
  readonly questionIds: ReadonlyArray<string>;
}

/** A new named collection has a visible title and private ownership. */
export interface CreateQuestionCollectionRequest extends QuestionCollectionReplaceRequest {
  readonly title: string;
}

/** A personal saved search retains current D1 filter meaning, never a page cursor. */
export interface SavedProblemSearchReplaceRequest {
  readonly title: string;
  readonly filter: CatalogSearchFilter;
}

/** Browser capability used by Library and the shared problem picker. */
export interface ProblemCurationClient {
  readonly listQuestionCollections: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<QuestionCollectionSummaryView>>;
  readonly getQuestionCollection: (
    collection: QuestionCollectionReference,
  ) => Promise<RevisionedQuestionCollection>;
  readonly listQuestionCollectionMembers: (
    collection: QuestionCollectionReference,
    cursor?: string,
    pageSize?: number,
  ) => Promise<QuestionCollectionMembersPage>;
  readonly createQuestionCollection: (
    request: CreateQuestionCollectionRequest,
  ) => Promise<RevisionedQuestionCollection>;
  readonly replaceQuestionCollection: (
    collection: QuestionCollectionReference,
    request: QuestionCollectionReplaceRequest,
    etag: ProblemCurationEtag,
  ) => Promise<RevisionedQuestionCollection>;
  readonly deleteQuestionCollection: (
    collection: QuestionCollectionReference,
    etag: ProblemCurationEtag,
  ) => Promise<void>;
  readonly listSavedProblemSearches: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<SavedProblemSearchView>>;
  readonly getSavedProblemSearch: (
    search: SavedProblemSearchReference,
  ) => Promise<RevisionedSavedProblemSearch>;
  readonly createSavedProblemSearch: (
    request: SavedProblemSearchReplaceRequest,
  ) => Promise<RevisionedSavedProblemSearch>;
  readonly replaceSavedProblemSearch: (
    search: SavedProblemSearchReference,
    request: SavedProblemSearchReplaceRequest,
    etag: ProblemCurationEtag,
  ) => Promise<RevisionedSavedProblemSearch>;
  readonly deleteSavedProblemSearch: (
    search: SavedProblemSearchReference,
    etag: ProblemCurationEtag,
  ) => Promise<void>;
}
