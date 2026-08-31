// problem_curation_model.ts - browser state rules for live question curation.

import type { QuestionCollectionMemberView } from "../../../generated/api/QuestionCollectionMemberView";
import type { QuestionCollectionReference } from "../../../generated/api/QuestionCollectionReference";
import type { QuestionCollectionSummaryView } from "../../../generated/api/QuestionCollectionSummaryView";
import type { SavedQuestionSearchReference } from "../../../generated/api/SavedQuestionSearchReference";
import type { SavedProblemSearchView } from "../../../generated/api/SavedProblemSearchView";
import type { CatalogSearchFilter } from "../../../generated/api/CatalogSearchFilter";
import type { ProblemCurationEtag } from "../../api/problem_curation";
import type { AuthenticatedSession } from "../../api/contracts";
import { normalizeQuestionIdSyntax } from "../../question_id";
import type { ProblemPickerSource } from "../problem_picker/problem_picker_model";
import {
  EMPTY_CATALOG_QUERY,
  normalizeCatalogBrowseQuery,
  type CatalogBrowseQuery,
} from "../../pages/library_page_model";

/** One bounded page from the collection or saved-search live API. */
export interface ProblemCurationPage<T> {
  readonly items: ReadonlyArray<T>;
  readonly nextCursor: string | null;
}

export interface RevisionedCurationValue<T> {
  readonly value: T;
  readonly etag: ProblemCurationEtag;
}

/** A complete edit-number-checked collection replacement. */
export interface QuestionCollectionReplacement {
  readonly reference: QuestionCollectionReference | null;
  readonly title: string;
  readonly questionIds: ReadonlyArray<string>;
  readonly editNumber: string | null;
}

/** A complete edit-number-checked saved-search replacement. */
export interface SavedProblemSearchReplacement {
  readonly reference: SavedQuestionSearchReference | null;
  readonly title: string;
  readonly query: CatalogBrowseQuery;
  /** Existing searches retain their exact D1 filter while a title changes. */
  readonly filter: CatalogSearchFilter | null;
  readonly editNumber: string | null;
}

/** One destructive intent paired with the exact edit number already visible in the UI. */
export type ProblemCurationDeletion =
  | {
      readonly kind: "collection";
      readonly reference: QuestionCollectionReference;
      readonly title: string;
      readonly editNumber: ProblemCurationEtag;
      readonly heading: string;
      readonly consequence: string;
      readonly confirmLabel: "Delete collection";
    }
  | {
      readonly kind: "savedSearch";
      readonly reference: SavedQuestionSearchReference;
      readonly title: string;
      readonly editNumber: ProblemCurationEtag;
      readonly heading: string;
      readonly consequence: string;
      readonly confirmLabel: "Delete saved search";
    };

export interface ProblemCurationConfirmationPresentation {
  readonly labelledBy: "curation-delete-heading";
  readonly describedBy: "curation-delete-consequence";
  readonly heading: string;
  readonly consequence: string;
  readonly actions: readonly [
    { readonly kind: "cancel"; readonly label: "Cancel"; readonly initial: true },
    { readonly kind: "confirm"; readonly label: string; readonly initial: false },
  ];
}

/** Browser route boundary for private Instructor Question Collections. */
export interface ProblemCurationRepository {
  readonly getCollection: (
    reference: QuestionCollectionReference,
  ) => Promise<RevisionedCurationValue<QuestionCollectionSummaryView>>;
  readonly listCollections: (
    cursor: string | null,
  ) => Promise<ProblemCurationPage<QuestionCollectionSummaryView>>;
  readonly listCollectionMembers: (
    reference: QuestionCollectionReference,
    cursor: string | null,
  ) => Promise<
    ProblemCurationPage<QuestionCollectionMemberView> & { readonly etag: ProblemCurationEtag }
  >;
  readonly replaceCollection: (
    replacement: QuestionCollectionReplacement,
  ) => Promise<RevisionedCurationValue<QuestionCollectionSummaryView>>;
  readonly deleteCollection: (
    reference: QuestionCollectionReference,
    editNumber: string,
  ) => Promise<void>;
  readonly listSavedSearches: (
    cursor: string | null,
  ) => Promise<ProblemCurationPage<SavedProblemSearchView>>;
  readonly replaceSavedSearch: (
    replacement: SavedProblemSearchReplacement,
  ) => Promise<RevisionedCurationValue<SavedProblemSearchView>>;
  readonly getSavedSearch: (
    reference: SavedQuestionSearchReference,
  ) => Promise<RevisionedCurationValue<SavedProblemSearchView>>;
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

export interface CollectionDraft {
  readonly reference: QuestionCollectionReference | null;
  readonly title: string;
  /** Exact strong ETag observed with the current complete collection projection. */
  readonly editNumber: string | null;
  readonly questionIds: ReadonlyArray<string>;
}

export const EMPTY_COLLECTION_DRAFT: CollectionDraft = {
  reference: null,
  title: "",
  editNumber: null,
  questionIds: [],
};

/** Converts one decoded edit number into the strong validator sent back as If-Match. */
export function curationEtagFromObservedEditNumber(editNumber: string): ProblemCurationEtag {
  if (!/^[1-9][0-9]*$/u.test(editNumber) || BigInt(editNumber) > 9_223_372_036_854_775_807n) {
    throw new Error("Use the current positive curation edit number before changing this item.");
  }
  return `"${editNumber}"`;
}

/** Builds the shared picker sources from private named collections. */
export function problemCurationPickerSources(
  collections: ReadonlyArray<QuestionCollectionSummaryView>,
  mayMutatePersonalCuration: boolean,
): ReadonlyArray<ProblemPickerSource> {
  return [
    { kind: "catalog", label: "Current library" },
    ...(mayMutatePersonalCuration ? ([{ kind: "mine", label: "My published questions" }] as const) : []),
    ...collections
      .map((collection) => ({
        kind: "collection" as const,
        label: collection.title,
        collection: collection.reference,
      })),
  ];
}

/** Retains a saved search's observed edit number and exact filter meaning for replacement. */
export function savedSearchReplacementFromObserved(
  search: SavedProblemSearchView | undefined,
  title: string,
  query: CatalogBrowseQuery,
): SavedProblemSearchReplacement {
  return {
    reference: search?.reference ?? null,
    title,
    query: savedSearchQuery(query),
    filter: search?.filter ?? null,
    editNumber: search === undefined ? null : curationEtagFromObservedEditNumber(search.editNumber),
  };
}

/** Names the collection and durable consequence before an owner confirms deletion. */
export function collectionDeletionFromObserved(
  collection: QuestionCollectionSummaryView,
): ProblemCurationDeletion {
  return {
    kind: "collection",
    reference: collection.reference,
    title: collection.title,
    editNumber: curationEtagFromObservedEditNumber(collection.editNumber),
    heading: `Delete collection "${collection.title}"?`,
    consequence:
      "Deleting this collection removes its saved ordered question list. Published questions remain available in the Library.",
    confirmLabel: "Delete collection",
  };
}

/** Names the saved search and durable consequence before its owner confirms deletion. */
export function savedSearchDeletionFromObserved(
  search: SavedProblemSearchView,
): ProblemCurationDeletion {
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
export function problemCurationConfirmationPresentation(
  deletion: ProblemCurationDeletion,
): ProblemCurationConfirmationPresentation {
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

export function mayEditOpenedQuestionCollection(
  mayMutatePersonalCuration: boolean,
): boolean {
  return mayMutatePersonalCuration;
}

/** Builds a whole-list edit from the currently authorized immutable members. */
export function collectionDraftFrom(
  collection: QuestionCollectionSummaryView,
  members: ReadonlyArray<QuestionCollectionMemberView>,
): CollectionDraft {
  return {
    reference: collection.reference,
    title: collection.title,
    editNumber: collection.editNumber,
    questionIds: canonicalQuestionIds(members.map((member) => member.questionId)),
  };
}

/** Retains existing order and appends each newly chosen public Question ID once. */
export function appendCollectionQuestionIds(
  current: ReadonlyArray<string>,
  additions: ReadonlyArray<string>,
): ReadonlyArray<string> {
  return canonicalQuestionIds([...current, ...additions]);
}

/** Removes one selected public Question ID from the whole-list draft. */
export function removeCollectionQuestionId(
  current: ReadonlyArray<string>,
  questionId: string,
): ReadonlyArray<string> {
  const canonical = canonicalQuestionId(questionId);
  return canonicalQuestionIds(current).filter((candidate) => candidate !== canonical);
}

/** Moves one visible collection member while preserving all other order. */
export function moveCollectionQuestionId(
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
export function savedSearchQuery(query: CatalogBrowseQuery): CatalogBrowseQuery {
  return normalizeCatalogBrowseQuery(query);
}

/** Converts the one-value Library controls into the D1 saved-search contract. */
export function catalogSearchFilterFromLibraryQuery(
  query: CatalogBrowseQuery,
): CatalogSearchFilter {
  const normalized = savedSearchQuery(query);
  const taxonomy = normalized.taxonomy;
  const taxonomySeparator = taxonomy === null ? -1 : taxonomy.indexOf(":");
  if (taxonomy !== null && (taxonomySeparator < 1 || taxonomySeparator === taxonomy.length - 1)) {
    throw new Error("Choose a complete topic before saving this search.");
  }
  return {
    text: normalized.search === "" ? null : normalized.search,
    bylines: normalized.byline === null ? [] : [normalized.byline],
    backends:
      normalized.backend === null ? [] : ([normalized.backend] as CatalogSearchFilter["backends"]),
    tags: normalized.tag === null ? [] : [normalized.tag],
    question_types:
      normalized.questionType === null
        ? []
        : ([normalized.questionType] as CatalogSearchFilter["question_types"]),
    taxonomy:
      taxonomy === null
        ? []
        : [
            {
              scheme: taxonomy.slice(0, taxonomySeparator),
              code: taxonomy.slice(taxonomySeparator + 1),
            },
          ],
    capabilities:
      normalized.capability === null
        ? []
        : ([normalized.capability] as CatalogSearchFilter["capabilities"]),
    licenses:
      normalized.license === null ? [] : ([normalized.license] as CatalogSearchFilter["licenses"]),
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

/** Reruns a saved search against the current catalog, always from its first page. */
export function libraryQueryFromSavedSearch(search: SavedProblemSearchView): CatalogBrowseQuery {
  const filter = search.filter;
  return {
    ...EMPTY_CATALOG_QUERY,
    search: filter.text ?? "",
    byline: filter.bylines[0] ?? null,
    backend: filter.backends[0] ?? null,
    tag: filter.tags[0] ?? null,
    questionType: filter.question_types[0] ?? null,
    taxonomy:
      filter.taxonomy[0] === undefined
        ? null
        : `${filter.taxonomy[0].scheme}:${filter.taxonomy[0].code}`,
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
