// Browser-owned command shapes for the server-owned D2 curation aggregate.

import type { CatalogSearchFilter } from "../../generated/api/CatalogSearchFilter";
import type { ProblemCollectionMemberView } from "../../generated/api/ProblemCollectionMemberView";
import type { ProblemCollectionReference } from "../../generated/api/ProblemCollectionReference";
import type { ProblemCollectionSummaryView } from "../../generated/api/ProblemCollectionSummaryView";
import type { ProblemCollectionVisibility } from "../../generated/api/ProblemCollectionVisibility";
import type { SavedProblemSearchReference } from "../../generated/api/SavedProblemSearchReference";
import type { SavedProblemSearchView } from "../../generated/api/SavedProblemSearchView";
import type { CursorPage } from "./contracts";

/** Strong server ETag that callers retain unchanged for a subsequent mutation. */
export type ProblemCurationEtag = string;

/** One current collection projection paired with its authoritative revision ETag. */
export interface RevisionedProblemCollection {
  readonly collection: ProblemCollectionSummaryView;
  readonly etag: ProblemCurationEtag;
}

/** One current saved-search projection paired with its authoritative revision ETag. */
export interface RevisionedSavedProblemSearch {
  readonly search: SavedProblemSearchView;
  readonly etag: ProblemCurationEtag;
}

/** A bounded members page tied to the collection revision returned in its ETag. */
export interface ProblemCollectionMembersPage {
  readonly page: CursorPage<ProblemCollectionMemberView>;
  readonly etag: ProblemCurationEtag;
}

/** Complete collection state submitted atomically; member order is meaningful. */
export interface ProblemCollectionReplaceRequest {
  readonly title?: string;
  readonly visibility?: ProblemCollectionVisibility;
  readonly questionIds: ReadonlyArray<string>;
}

/** A new named collection has a visible title and an explicit sharing choice. */
export interface CreateProblemCollectionRequest extends ProblemCollectionReplaceRequest {
  readonly title: string;
  readonly visibility: ProblemCollectionVisibility;
}

/** A personal saved search retains current D1 filter meaning, never a page cursor. */
export interface SavedProblemSearchReplaceRequest {
  readonly title: string;
  readonly filter: CatalogSearchFilter;
}

/** Browser capability used by Library and the shared problem picker. */
export interface ProblemCurationClient {
  readonly listProblemCollections: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<ProblemCollectionSummaryView>>;
  readonly getProblemCollection: (
    collection: ProblemCollectionReference,
  ) => Promise<RevisionedProblemCollection>;
  /** Creates or returns the Account's persistent Favorites aggregate through a state transition. */
  readonly ensureFavorites: () => Promise<RevisionedProblemCollection>;
  readonly listProblemCollectionMembers: (
    collection: ProblemCollectionReference,
    cursor?: string,
    pageSize?: number,
  ) => Promise<ProblemCollectionMembersPage>;
  readonly createProblemCollection: (
    request: CreateProblemCollectionRequest,
  ) => Promise<RevisionedProblemCollection>;
  readonly replaceProblemCollection: (
    collection: ProblemCollectionReference,
    request: ProblemCollectionReplaceRequest,
    etag: ProblemCurationEtag,
  ) => Promise<RevisionedProblemCollection>;
  readonly replaceFavorites: (
    request: ProblemCollectionReplaceRequest,
    etag: ProblemCurationEtag,
  ) => Promise<RevisionedProblemCollection>;
  readonly deleteProblemCollection: (
    collection: ProblemCollectionReference,
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
