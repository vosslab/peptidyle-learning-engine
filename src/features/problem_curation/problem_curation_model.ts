// problem_curation_model.ts - browser state rules for live question curation.

import type { ProblemCollectionMemberView } from "../../../generated/api/ProblemCollectionMemberView";
import type { ProblemCollectionReference } from "../../../generated/api/ProblemCollectionReference";
import type { ProblemCollectionSummaryView } from "../../../generated/api/ProblemCollectionSummaryView";
import type { ProblemCollectionVisibility } from "../../../generated/api/ProblemCollectionVisibility";
import type { SavedProblemSearchReference } from "../../../generated/api/SavedProblemSearchReference";
import type { SavedProblemSearchView } from "../../../generated/api/SavedProblemSearchView";
import type { CatalogSearchFilter } from "../../../generated/api/CatalogSearchFilter";
import type { ProblemCurationEtag } from "../../api/problem_curation";
import type { AuthSession } from "../../api/contracts";
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

/** A complete, revision-checked collection replacement. */
export interface ProblemCollectionReplacement {
  readonly reference: ProblemCollectionReference | null;
  readonly kind: "favorites" | "named";
  readonly title: string;
  readonly visibility: ProblemCollectionVisibility;
  readonly questionIds: ReadonlyArray<string>;
  readonly revision: string | null;
}

/** A complete, revision-checked saved-search replacement. */
export interface SavedProblemSearchReplacement {
  readonly reference: SavedProblemSearchReference | null;
  readonly title: string;
  readonly query: CatalogBrowseQuery;
  /** Existing searches retain their exact D1 filter while a title changes. */
  readonly filter: CatalogSearchFilter | null;
  readonly revision: string | null;
}

/** One destructive intent paired with the exact revision already visible in the UI. */
export type ProblemCurationDeletion =
  | {
      readonly kind: "collection";
      readonly reference: ProblemCollectionReference;
      readonly title: string;
      readonly revision: ProblemCurationEtag;
      readonly heading: string;
      readonly consequence: string;
      readonly confirmLabel: "Delete collection";
    }
  | {
      readonly kind: "savedSearch";
      readonly reference: SavedProblemSearchReference;
      readonly title: string;
      readonly revision: ProblemCurationEtag;
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

/** Browser route boundary for personal and institution curation. */
export interface ProblemCurationRepository {
  readonly ensureFavorites: () => Promise<RevisionedCurationValue<ProblemCollectionSummaryView>>;
  readonly getCollection: (
    reference: ProblemCollectionReference,
  ) => Promise<RevisionedCurationValue<ProblemCollectionSummaryView>>;
  readonly listCollections: (
    cursor: string | null,
  ) => Promise<ProblemCurationPage<ProblemCollectionSummaryView>>;
  readonly listCollectionMembers: (
    reference: ProblemCollectionReference,
    cursor: string | null,
  ) => Promise<
    ProblemCurationPage<ProblemCollectionMemberView> & { readonly etag: ProblemCurationEtag }
  >;
  readonly replaceCollection: (
    replacement: ProblemCollectionReplacement,
  ) => Promise<RevisionedCurationValue<ProblemCollectionSummaryView>>;
  readonly deleteCollection: (
    reference: ProblemCollectionReference,
    revision: string,
  ) => Promise<void>;
  readonly listSavedSearches: (
    cursor: string | null,
  ) => Promise<ProblemCurationPage<SavedProblemSearchView>>;
  readonly replaceSavedSearch: (
    replacement: SavedProblemSearchReplacement,
  ) => Promise<RevisionedCurationValue<SavedProblemSearchView>>;
  readonly getSavedSearch: (
    reference: SavedProblemSearchReference,
  ) => Promise<RevisionedCurationValue<SavedProblemSearchView>>;
  readonly deleteSavedSearch: (
    reference: SavedProblemSearchReference,
    revision: string,
  ) => Promise<void>;
}

export type CurationNotice =
  | { readonly kind: "idle" }
  | { readonly kind: "working"; readonly text: string }
  | { readonly kind: "success"; readonly text: string }
  | { readonly kind: "error"; readonly text: string };

export interface CollectionDraft {
  readonly reference: ProblemCollectionReference | null;
  readonly kind: "favorites" | "named";
  readonly title: string;
  readonly visibility: ProblemCollectionVisibility;
  /** Exact strong ETag observed with the current complete collection projection. */
  readonly revision: string | null;
  readonly questionIds: ReadonlyArray<string>;
}

export const EMPTY_COLLECTION_DRAFT: CollectionDraft = {
  reference: null,
  kind: "named",
  title: "",
  visibility: "private",
  revision: null,
  questionIds: [],
};

/** Converts one decoded revision into the strong validator sent back as If-Match. */
export function curationEtagFromObservedRevision(revision: string): ProblemCurationEtag {
  if (!/^[1-9][0-9]*$/u.test(revision) || BigInt(revision) > 9_223_372_036_854_775_807n) {
    throw new Error("Use the current positive curation revision before changing this item.");
  }
  return `"${revision}"`;
}

/** Builds the shared picker sources without duplicating the dedicated Favorites source. */
export function problemCurationPickerSources(
  collections: ReadonlyArray<ProblemCollectionSummaryView>,
  mayMutatePersonalCuration: boolean,
): ReadonlyArray<ProblemPickerSource> {
  return [
    { kind: "catalog", label: "Current library" },
    ...(mayMutatePersonalCuration
      ? ([
          { kind: "mine", label: "My published questions" },
          { kind: "favorites", label: "Favorites" },
        ] as const)
      : []),
    ...collections
      .filter((collection) => collection.kind === "named")
      .map((collection) => ({
        kind: "collection" as const,
        label: collection.title,
        collection: collection.reference,
      })),
  ];
}

/** Retains a saved search's observed revision and exact filter meaning for replacement. */
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
    revision: search === undefined ? null : curationEtagFromObservedRevision(search.revision),
  };
}

/** Names the collection and durable consequence before an owner confirms deletion. */
export function collectionDeletionFromObserved(
  collection: ProblemCollectionSummaryView,
): ProblemCurationDeletion {
  if (collection.kind !== "named" || collection.access !== "owner") {
    throw new Error("Choose one of your named collections before deleting it.");
  }
  return {
    kind: "collection",
    reference: collection.reference,
    title: collection.title,
    revision: curationEtagFromObservedRevision(collection.revision),
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
    revision: curationEtagFromObservedRevision(search.revision),
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
export function mayMutatePersonalCuration(session: AuthSession | undefined): boolean {
  return session?.user.roles.includes("instructor") ?? false;
}

/** Institution-reader access is deliberately a browse/reuse projection. */
export function mayEditOpenedProblemCollection(
  mayMutatePersonalCuration: boolean,
  access: ProblemCollectionSummaryView["access"],
): boolean {
  return mayMutatePersonalCuration && access === "owner";
}

/** Builds a whole-list edit from the currently authorized immutable members. */
export function collectionDraftFrom(
  collection: ProblemCollectionSummaryView,
  members: ReadonlyArray<ProblemCollectionMemberView>,
): CollectionDraft {
  return {
    reference: collection.reference,
    kind: collection.kind,
    title: collection.title,
    visibility: collection.visibility,
    revision: collection.revision,
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
    response_families:
      normalized.responseFamily === null
        ? []
        : ([normalized.responseFamily] as CatalogSearchFilter["response_families"]),
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
    responseFamily: filter.response_families[0] ?? null,
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
