// question_curation_model.ts - browser state rules for live Question Curation.

import type { QuestionFolderEntryView } from "../../../generated/api/QuestionFolderEntryView";
import type { QuestionFolderReference } from "../../../generated/api/QuestionFolderReference";
import type { QuestionFolderSummaryView } from "../../../generated/api/QuestionFolderSummaryView";
import type { SavedQuestionSearchReference } from "../../../generated/api/SavedQuestionSearchReference";
import type { SavedQuestionSearchView } from "../../../generated/api/SavedQuestionSearchView";
import type { QuestionSearchFilter } from "../../../generated/api/QuestionSearchFilter";
import type { QuestionCurationEtag } from "../../api/question_curation";
import type { AuthenticatedSession } from "../../api/contracts";
import { normalizeQuestionIdSyntax } from "../../question_id";
import type { QuestionPickerSource } from "../question_picker/question_picker_model";
import {
  EMPTY_QUESTION_SEARCH_QUERY,
  normalizeQuestionSearchQuery,
  type QuestionSearchQuery,
} from "../../pages/library_page_model";

/** One bounded page from the Folder or saved-search live API. */
export interface QuestionCurationPage<T> {
  readonly items: ReadonlyArray<T>;
  readonly nextCursor: string | null;
}

export interface RevisionedCurationValue<T> {
  readonly value: T;
  readonly etag: QuestionCurationEtag;
}

/** A complete edit-number-checked Question Folder replacement. */
export interface QuestionFolderReplacement {
  readonly reference: QuestionFolderReference | null;
  readonly title: string;
  readonly questionIds: ReadonlyArray<string>;
  readonly editNumber: string | null;
}

/** A complete edit-number-checked saved-search replacement. */
export interface SavedQuestionSearchReplacement {
  readonly reference: SavedQuestionSearchReference | null;
  readonly title: string;
  readonly query: QuestionSearchQuery;
  /** Existing searches retain their exact D1 filter while a title changes. */
  readonly filter: QuestionSearchFilter | null;
  readonly editNumber: string | null;
}

/** One destructive intent paired with the exact edit number already visible in the UI. */
export type QuestionCurationDeletion =
  | {
      readonly kind: "folder";
      readonly reference: QuestionFolderReference;
      readonly title: string;
      readonly editNumber: QuestionCurationEtag;
      readonly heading: string;
      readonly consequence: string;
      readonly confirmLabel: "Delete Question Folder";
    }
  | {
      readonly kind: "savedSearch";
      readonly reference: SavedQuestionSearchReference;
      readonly title: string;
      readonly editNumber: QuestionCurationEtag;
      readonly heading: string;
      readonly consequence: string;
      readonly confirmLabel: "Delete saved search";
    };

export interface QuestionCurationConfirmationPresentation {
  readonly labelledBy: "curation-delete-heading";
  readonly describedBy: "curation-delete-consequence";
  readonly heading: string;
  readonly consequence: string;
  readonly actions: readonly [
    { readonly kind: "cancel"; readonly label: "Cancel"; readonly initial: true },
    { readonly kind: "confirm"; readonly label: string; readonly initial: false },
  ];
}

/** Browser route boundary for private Instructor Question Folders. */
export interface QuestionCurationRepository {
  readonly getFolder: (
    reference: QuestionFolderReference,
  ) => Promise<RevisionedCurationValue<QuestionFolderSummaryView>>;
  readonly listFolders: (
    cursor: string | null,
  ) => Promise<QuestionCurationPage<QuestionFolderSummaryView>>;
  readonly listFolderEntries: (
    reference: QuestionFolderReference,
    cursor: string | null,
  ) => Promise<
    QuestionCurationPage<QuestionFolderEntryView> & { readonly etag: QuestionCurationEtag }
  >;
  readonly replaceFolder: (
    replacement: QuestionFolderReplacement,
  ) => Promise<RevisionedCurationValue<QuestionFolderSummaryView>>;
  readonly deleteFolder: (reference: QuestionFolderReference, editNumber: string) => Promise<void>;
  readonly listSavedSearches: (
    cursor: string | null,
  ) => Promise<QuestionCurationPage<SavedQuestionSearchView>>;
  readonly replaceSavedSearch: (
    replacement: SavedQuestionSearchReplacement,
  ) => Promise<RevisionedCurationValue<SavedQuestionSearchView>>;
  readonly getSavedSearch: (
    reference: SavedQuestionSearchReference,
  ) => Promise<RevisionedCurationValue<SavedQuestionSearchView>>;
  readonly deleteSavedSearch: (
    reference: SavedQuestionSearchReference,
    editNumber: string,
  ) => Promise<void>;
}

export type CurationNotice =
  | { readonly kind: "idle" }
  | { readonly kind: "working"; readonly text: string }
  | { readonly kind: "success"; readonly text: string }
  | { readonly kind: "error"; readonly text: string };

export interface FolderDraft {
  readonly reference: QuestionFolderReference | null;
  readonly title: string;
  /** Exact strong ETag observed with the current complete Question Folder projection. */
  readonly editNumber: string | null;
  readonly questionIds: ReadonlyArray<string>;
}

export const EMPTY_FOLDER_DRAFT: FolderDraft = {
  reference: null,
  title: "",
  editNumber: null,
  questionIds: [],
};

/** Converts one decoded edit number into the strong validator sent back as If-Match. */
export function curationEtagFromObservedEditNumber(editNumber: string): QuestionCurationEtag {
  if (!/^[1-9][0-9]*$/u.test(editNumber) || BigInt(editNumber) > 9_223_372_036_854_775_807n) {
    throw new Error("Use the current positive curation edit number before changing this item.");
  }
  return `"${editNumber}"`;
}

/** Builds the shared picker sources from private named Question Folders. */
export function questionCurationPickerSources(
  folders: ReadonlyArray<QuestionFolderSummaryView>,
  mayMutatePersonalCuration: boolean,
): ReadonlyArray<QuestionPickerSource> {
  return [
    { kind: "library", label: "Current library" },
    ...(mayMutatePersonalCuration ? ([{ kind: "mine", label: "My Questions" }] as const) : []),
    ...folders.map((folder) => ({
      kind: "folder" as const,
      label: folder.title,
      folder: folder.reference,
    })),
  ];
}

/** Retains a saved search's observed edit number and exact filter meaning for replacement. */
export function savedSearchReplacementFromObserved(
  search: SavedQuestionSearchView | undefined,
  title: string,
  query: QuestionSearchQuery,
): SavedQuestionSearchReplacement {
  return {
    reference: search?.reference ?? null,
    title,
    query: savedSearchQuery(query),
    filter: search?.filter ?? null,
    editNumber: search === undefined ? null : curationEtagFromObservedEditNumber(search.editNumber),
  };
}

/** Names the Folder and durable consequence before an owner confirms deletion. */
export function folderDeletionFromObserved(
  folder: QuestionFolderSummaryView,
): QuestionCurationDeletion {
  return {
    kind: "folder",
    reference: folder.reference,
    title: folder.title,
    editNumber: curationEtagFromObservedEditNumber(folder.editNumber),
    heading: `Delete Question Folder "${folder.title}"?`,
    consequence:
      "Deleting this Question Folder removes its saved ordered Question list. Published Questions remain available in the Question Library.",
    confirmLabel: "Delete Question Folder",
  };
}

/** Names the saved search and durable consequence before its owner confirms deletion. */
export function savedSearchDeletionFromObserved(
  search: SavedQuestionSearchView,
): QuestionCurationDeletion {
  return {
    kind: "savedSearch",
    reference: search.reference,
    title: search.title,
    editNumber: curationEtagFromObservedEditNumber(search.editNumber),
    heading: `Delete saved search "${search.title}"?`,
    consequence:
      "Deleting this saved search removes the shortcut. Current Library questions and filters remain available.",
    confirmLabel: "Delete saved search",
  };
}

/** Supplies one accessible name, description, and Cancel-first action order to the dialog. */
export function questionCurationConfirmationPresentation(
  deletion: QuestionCurationDeletion,
): QuestionCurationConfirmationPresentation {
  return {
    labelledBy: "curation-delete-heading",
    describedBy: "curation-delete-consequence",
    heading: deletion.heading,
    consequence: deletion.consequence,
    actions: [
      { kind: "cancel", label: "Cancel", initial: true },
      { kind: "confirm", label: deletion.confirmLabel, initial: false },
    ],
  };
}

/** The current authenticated Instructor authority unlocks personal curation. */
export function mayMutatePersonalCuration(session: AuthenticatedSession | undefined): boolean {
  return session?.account.role === "instructor";
}

export function mayEditOpenedQuestionFolder(mayMutatePersonalCuration: boolean): boolean {
  return mayMutatePersonalCuration;
}

/** Builds a whole-list edit from the currently authorized immutable members. */
export function folderDraftFrom(
  folder: QuestionFolderSummaryView,
  entries: ReadonlyArray<QuestionFolderEntryView>,
): FolderDraft {
  return {
    reference: folder.reference,
    title: folder.title,
    editNumber: folder.editNumber,
    questionIds: canonicalQuestionIds(entries.map((entry) => entry.questionId)),
  };
}

/** Retains existing order and appends each newly chosen public Question ID once. */
export function appendFolderQuestionIds(
  current: ReadonlyArray<string>,
  additions: ReadonlyArray<string>,
): ReadonlyArray<string> {
  return canonicalQuestionIds([...current, ...additions]);
}

/** Removes one selected public Question ID from the whole-list draft. */
export function removeFolderQuestionId(
  current: ReadonlyArray<string>,
  questionId: string,
): ReadonlyArray<string> {
  const canonical = canonicalQuestionId(questionId);
  return canonicalQuestionIds(current).filter((candidate) => candidate !== canonical);
}

/** Moves one visible Question Folder Entry while preserving all other order. */
export function moveFolderQuestionId(
  current: ReadonlyArray<string>,
  index: number,
  direction: -1 | 1,
): ReadonlyArray<string> {
  const ids = [...canonicalQuestionIds(current)];
  const destination = index + direction;
  if (index < 0 || destination < 0 || destination >= ids.length) return ids;
  const item = ids[index];
  const adjacent = ids[destination];
  if (item === undefined || adjacent === undefined) return ids;
  ids[index] = adjacent;
  ids[destination] = item;
  return ids;
}

/** Normalizes the current Library query at the moment an instructor saves it. */
export function savedSearchQuery(query: QuestionSearchQuery): QuestionSearchQuery {
  return normalizeQuestionSearchQuery(query);
}

/** Converts the one-value Library controls into the D1 saved-search contract. */
export function questionSearchFilterFromLibraryQuery(
  query: QuestionSearchQuery,
): QuestionSearchFilter {
  const normalized = savedSearchQuery(query);
  const classification = normalized.classification;
  const classificationSeparator = classification === null ? -1 : classification.indexOf(":");
  if (
    classification !== null &&
    (classificationSeparator < 1 || classificationSeparator === classification.length - 1)
  ) {
    throw new Error("Choose a complete topic before saving this search.");
  }
  return {
    text: normalized.search === "" ? null : normalized.search,
    bylines: normalized.byline === null ? [] : [normalized.byline],
    backends:
      normalized.backend === null ? [] : ([normalized.backend] as QuestionSearchFilter["backends"]),
    tags: normalized.tag === null ? [] : [normalized.tag],
    question_types:
      normalized.questionType === null
        ? []
        : ([normalized.questionType] as QuestionSearchFilter["question_types"]),
    classifications:
      classification === null
        ? []
        : [
            {
              system: classification.slice(0, classificationSeparator),
              code: classification.slice(classificationSeparator + 1),
            },
          ],
    capabilities:
      normalized.capability === null
        ? []
        : ([normalized.capability] as QuestionSearchFilter["capabilities"]),
    licenses:
      normalized.license === null ? [] : ([normalized.license] as QuestionSearchFilter["licenses"]),
    evidence:
      normalized.evidence === null
        ? "any"
        : normalized.evidence === "available"
          ? "available"
          : "unavailable",
    used_in_my_courses: normalized.usedInMyCourses === "used" ? "used" : "any",
    authorship: normalized.authorship,
  };
}

/** Reruns a saved search against the current Question Library, always from its first page. */
export function libraryQueryFromSavedSearch(search: SavedQuestionSearchView): QuestionSearchQuery {
  const filter = search.filter;
  return {
    ...EMPTY_QUESTION_SEARCH_QUERY,
    search: filter.text ?? "",
    byline: filter.bylines[0] ?? null,
    backend: filter.backends[0] ?? null,
    tag: filter.tags[0] ?? null,
    questionType: filter.question_types[0] ?? null,
    classification:
      filter.classifications[0] === undefined
        ? null
        : `${filter.classifications[0].system}:${filter.classifications[0].code}`,
    capability: filter.capabilities[0] ?? null,
    license: filter.licenses[0] ?? null,
    evidence: filter.evidence === "any" ? null : filter.evidence,
    usedInMyCourses: filter.used_in_my_courses === "any" ? null : filter.used_in_my_courses,
    authorship: filter.authorship,
  };
}

function canonicalQuestionId(value: string): string {
  const canonical = normalizeQuestionIdSyntax(value);
  if (canonical === null) throw new Error("Choose a canonical public Question ID.");
  return canonical;
}

function canonicalQuestionIds(values: ReadonlyArray<string>): ReadonlyArray<string> {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const value of values) {
    const canonical = canonicalQuestionId(value);
    if (seen.has(canonical)) continue;
    seen.add(canonical);
    result.push(canonical);
  }
  return result;
}
