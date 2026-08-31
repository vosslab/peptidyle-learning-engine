// Browser-owned command shapes for the server-owned Question Curation aggregate.

import type { QuestionSearchFilter } from "../../generated/api/QuestionSearchFilter";
import type { QuestionFolderEntryView } from "../../generated/api/QuestionFolderEntryView";
import type { QuestionFolderReference } from "../../generated/api/QuestionFolderReference";
import type { QuestionFolderSummaryView } from "../../generated/api/QuestionFolderSummaryView";
import type { SavedQuestionSearchReference } from "../../generated/api/SavedQuestionSearchReference";
import type { SavedQuestionSearchView } from "../../generated/api/SavedQuestionSearchView";
import type { CursorPage } from "./contracts";

/** Strong server ETag that callers retain unchanged for a subsequent mutation. */
export type QuestionCurationEtag = string;

/** One current Question Folder projection paired with its authoritative edit-number ETag. */
export interface RevisionedQuestionFolder {
  readonly folder: QuestionFolderSummaryView;
  readonly etag: QuestionCurationEtag;
}

/** One current saved-search projection paired with its authoritative edit-number ETag. */
export interface RevisionedSavedQuestionSearch {
  readonly search: SavedQuestionSearchView;
  readonly etag: QuestionCurationEtag;
}

/** A bounded Folder Entries page tied to the Folder edit number returned in its ETag. */
export interface QuestionFolderEntriesPage {
  readonly page: CursorPage<QuestionFolderEntryView>;
  readonly etag: QuestionCurationEtag;
}

/** Complete Question Folder state submitted atomically; entry order is meaningful. */
export interface QuestionFolderReplaceRequest {
  readonly title?: string;
  readonly questionIds: ReadonlyArray<string>;
}

/** A new named Question Folder has a visible title and private ownership. */
export interface CreateQuestionFolderRequest extends QuestionFolderReplaceRequest {
  readonly title: string;
}

/** A personal saved search retains current D1 filter meaning, never a page cursor. */
export interface SavedQuestionSearchReplaceRequest {
  readonly title: string;
  readonly filter: QuestionSearchFilter;
}

/** Browser capability used by Library and the shared Question Picker. */
export interface QuestionCurationClient {
  readonly listQuestionFolders: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<QuestionFolderSummaryView>>;
  readonly getQuestionFolder: (
    folder: QuestionFolderReference,
  ) => Promise<RevisionedQuestionFolder>;
  readonly listQuestionFolderEntries: (
    folder: QuestionFolderReference,
    cursor?: string,
    pageSize?: number,
  ) => Promise<QuestionFolderEntriesPage>;
  readonly createQuestionFolder: (
    request: CreateQuestionFolderRequest,
  ) => Promise<RevisionedQuestionFolder>;
  readonly replaceQuestionFolder: (
    folder: QuestionFolderReference,
    request: QuestionFolderReplaceRequest,
    etag: QuestionCurationEtag,
  ) => Promise<RevisionedQuestionFolder>;
  readonly deleteQuestionFolder: (
    folder: QuestionFolderReference,
    etag: QuestionCurationEtag,
  ) => Promise<void>;
  readonly listSavedQuestionSearches: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<SavedQuestionSearchView>>;
  readonly getSavedQuestionSearch: (
    search: SavedQuestionSearchReference,
  ) => Promise<RevisionedSavedQuestionSearch>;
  readonly createSavedQuestionSearch: (
    request: SavedQuestionSearchReplaceRequest,
  ) => Promise<RevisionedSavedQuestionSearch>;
  readonly replaceSavedQuestionSearch: (
    search: SavedQuestionSearchReference,
    request: SavedQuestionSearchReplaceRequest,
    etag: QuestionCurationEtag,
  ) => Promise<RevisionedSavedQuestionSearch>;
  readonly deleteSavedQuestionSearch: (
    search: SavedQuestionSearchReference,
    etag: QuestionCurationEtag,
  ) => Promise<void>;
}
