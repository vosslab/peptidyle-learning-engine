// problem_curation_panel.tsx - live collection, Favorites, and saved-search workspace.

import { For, Show, createSignal, onMount, type Accessor, type JSX } from "solid-js";

import type { ProblemCollectionMemberView } from "../../../generated/api/ProblemCollectionMemberView";
import { ProblemCurationConflictError } from "../../api/http_client";
import type { ProblemCollectionSummaryView } from "../../../generated/api/ProblemCollectionSummaryView";
import type { SavedProblemSearchView } from "../../../generated/api/SavedProblemSearchView";
import { MAX_PROBLEM_COLLECTION_MEMBERS } from "../../../generated/api/MAX_PROBLEM_COLLECTION_MEMBERS";
import {
  ProblemPicker,
  type ProblemPickerSelection,
  type ProblemPickerSourceRepository,
} from "../problem_picker";
import type { CatalogBrowseQuery } from "../../pages/library_page_model";
import "./problem_curation.css";
import { ProblemCurationConfirmationDialog } from "./problem_curation_confirmation";
import {
  EMPTY_COLLECTION_DRAFT,
  appendCollectionQuestionIds,
  collectionDraftFrom,
  collectionDeletionFromObserved,
  libraryQueryFromSavedSearch,
  mayEditOpenedProblemCollection,
  moveCollectionQuestionId,
  problemCurationPickerSources,
  removeCollectionQuestionId,
  savedSearchDeletionFromObserved,
  savedSearchReplacementFromObserved,
  type CollectionDraft,
  type CurationNotice,
  type ProblemCurationDeletion,
  type ProblemCurationPage,
  type ProblemCurationRepository,
} from "./problem_curation_model";

export interface ProblemCurationPanelProps {
  readonly repository: ProblemCurationRepository;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly query: Accessor<CatalogBrowseQuery>;
  readonly applyQuery: (query: CatalogBrowseQuery) => void;
  /** Instructor authority enables personal Favorites, collections, and saved searches. */
  readonly mayMutatePersonalCuration: boolean;
}

function noticeText(notice: CurationNotice): string {
  return notice.kind === "idle"
    ? "Choose a visible action to organize questions you can reuse."
    : notice.text;
}

function failureNotice(error: unknown, ordinary: string, conflict?: string): CurationNotice {
  if (error instanceof ProblemCurationConflictError) {
    return {
      kind: "error",
      text:
        conflict ??
        "Someone saved a newer version first. Reload curation, review the current version, then apply your retained draft.",
    };
  }
  return { kind: "error", text: ordinary };
}

function pageWith<T>(items: ReadonlyArray<T>, nextCursor: string | null): ProblemCurationPage<T> {
  return { items, nextCursor };
}

/**
 * Visible task-order curation: discover, stage a bounded selection, choose a
 * named destination, then receive a durable result or a clear recovery action.
 */
export function ProblemCurationPanel(props: ProblemCurationPanelProps): JSX.Element {
  type PendingDeletion = {
    readonly deletion: ProblemCurationDeletion;
    readonly trigger: HTMLButtonElement;
  };

  const [collections, setCollections] = createSignal<
    ProblemCurationPage<ProblemCollectionSummaryView>
  >(pageWith([], null));
  const [savedSearches, setSavedSearches] = createSignal<
    ProblemCurationPage<SavedProblemSearchView>
  >(pageWith([], null));
  const [draft, setDraft] = createSignal<CollectionDraft>(EMPTY_COLLECTION_DRAFT);
  const [stagedQuestionIds, setStagedQuestionIds] = createSignal<ReadonlyArray<string>>([]);
  const [notice, setNotice] = createSignal<CurationNotice>({ kind: "idle" });
  const [showPicker, setShowPicker] = createSignal(false);
  const [showCollectionForm, setShowCollectionForm] = createSignal(false);
  const [showSavedSearchForm, setShowSavedSearchForm] = createSignal(false);
  const [canEditDraft, setCanEditDraft] = createSignal(true);
  const [openedCollection, setOpenedCollection] = createSignal<ProblemCollectionSummaryView>();
  const [openedMembers, setOpenedMembers] = createSignal<
    ProblemCurationPage<ProblemCollectionMemberView>
  >(pageWith([], null));
  const [collectionTitle, setCollectionTitle] = createSignal("");
  const [collectionVisibility, setCollectionVisibility] = createSignal<"private" | "institution">(
    "private",
  );
  const [savedSearchTitle, setSavedSearchTitle] = createSignal("");
  const [editingSavedSearch, setEditingSavedSearch] = createSignal<
    SavedProblemSearchView | undefined
  >();
  const [pendingDeletion, setPendingDeletion] = createSignal<PendingDeletion>();
  const [deletionBusy, setDeletionBusy] = createSignal(false);
  let pickerTrigger: HTMLButtonElement | undefined;
  let collectionsHeading: HTMLHeadingElement | undefined;
  let savedSearchesHeading: HTMLHeadingElement | undefined;

  const favorite = (): ProblemCollectionSummaryView | undefined =>
    collections().items.find((collection) => collection.kind === "favorites");

  async function loadCollections(cursor: string | null, append: boolean): Promise<boolean> {
    try {
      const page = await props.repository.listCollections(cursor);
      setCollections((current) =>
        append ? pageWith([...current.items, ...page.items], page.nextCursor) : page,
      );
      return true;
    } catch (error: unknown) {
      setNotice(failureNotice(error, "Collections could not load. Try loading them again."));
      return false;
    }
  }

  async function ensureFavorites(): Promise<boolean> {
    try {
      const result = await props.repository.ensureFavorites();
      const favorites = result.value;
      setCollections((page) =>
        pageWith(
          [favorites, ...page.items.filter((item) => item.reference !== favorites.reference)],
          page.nextCursor,
        ),
      );
      return true;
    } catch (error: unknown) {
      setNotice(failureNotice(error, "Favorites could not load. Try loading the Library again."));
      return false;
    }
  }

  async function loadSavedSearches(cursor: string | null, append: boolean): Promise<boolean> {
    try {
      const page = await props.repository.listSavedSearches(cursor);
      setSavedSearches((current) =>
        append ? pageWith([...current.items, ...page.items], page.nextCursor) : page,
      );
      if (!append) {
        setEditingSavedSearch((current) =>
          current === undefined
            ? undefined
            : page.items.find((search) => search.reference === current.reference),
        );
      }
      return true;
    } catch (error: unknown) {
      setNotice(failureNotice(error, "Saved searches could not load. Try loading them again."));
      return false;
    }
  }

  async function reloadCuration(): Promise<void> {
    setNotice({ kind: "working", text: "Reloading current curation." });
    const results = await Promise.all([
      loadCollections(null, false),
      ...(props.mayMutatePersonalCuration
        ? [ensureFavorites(), loadSavedSearches(null, false)]
        : []),
    ]);
    if (results.some((loaded) => !loaded)) return;

    const opened = openedCollection();
    if (opened !== undefined && canEditDraft()) {
      try {
        const current = await props.repository.getCollection(opened.reference);
        setOpenedCollection(current.value);
        setDraft((local) =>
          local.reference === opened.reference ? { ...local, revision: current.etag } : local,
        );
        setNotice({
          kind: "success",
          text: `${current.value.title} is current. Your selected questions and edits are ready to review and save.`,
        });
        return;
      } catch (error: unknown) {
        setNotice(failureNotice(error, "The current collection could not reload. Try again."));
        return;
      }
    }
    setNotice({ kind: "success", text: "Current curation is loaded." });
  }

  async function loadCollection(collection: ProblemCollectionSummaryView): Promise<boolean> {
    setNotice({ kind: "working", text: `Loading ${collection.title}.` });
    try {
      const current = await props.repository.getCollection(collection.reference);
      setOpenedCollection(current.value);
      const mayEdit = mayEditOpenedProblemCollection(
        props.mayMutatePersonalCuration,
        current.value.access,
      );
      setCanEditDraft(mayEdit);
      if (!mayEdit) {
        const page = await props.repository.listCollectionMembers(collection.reference, null);
        setOpenedMembers(pageWith(page.items, page.nextCursor));
        setNotice({
          kind: "success",
          text: `${current.value.title} is available to browse and reuse. Its owner controls changes.`,
        });
        return true;
      }
      const loaded: ProblemCollectionMemberView[] = [];
      let etag = current.etag;
      let cursor: string | null = null;
      do {
        const page = await props.repository.listCollectionMembers(collection.reference, cursor);
        loaded.push(...page.items);
        etag = page.etag;
        cursor = page.nextCursor;
      } while (cursor !== null && loaded.length < MAX_PROBLEM_COLLECTION_MEMBERS);
      setDraft({
        ...collectionDraftFrom(current.value, loaded),
        revision: etag,
      });
      setCollectionTitle(current.value.title);
      setCollectionVisibility(current.value.visibility);
      setCanEditDraft(mayEdit);
      setOpenedMembers(pageWith(loaded, null));
      setShowCollectionForm(true);
      setNotice({
        kind: "success",
        text:
          current.value.access === "owner"
            ? `${current.value.title} is ready to edit.`
            : `${current.value.title} is available for question selection.`,
      });
      return true;
    } catch (error: unknown) {
      setNotice(
        failureNotice(
          error,
          "That collection could not load. Your current selection is still available.",
        ),
      );
      return false;
    }
  }

  async function loadMoreOpenedMembers(): Promise<void> {
    const collection = openedCollection();
    const cursor = openedMembers().nextCursor;
    if (collection === undefined || cursor === null) return;
    try {
      const page = await props.repository.listCollectionMembers(collection.reference, cursor);
      setOpenedMembers((current) => pageWith([...current.items, ...page.items], page.nextCursor));
    } catch (error: unknown) {
      setNotice(failureNotice(error, "More collection members could not load. Try again."));
    }
  }

  async function saveDraft(): Promise<void> {
    const current = draft();
    const title = collectionTitle().trim();
    if (title.length === 0) {
      setNotice({ kind: "error", text: "Name the collection before saving it." });
      return;
    }
    setNotice({ kind: "working", text: `Saving ${title}.` });
    try {
      const savedResult = await props.repository.replaceCollection({
        reference: current.reference,
        kind: current.kind,
        title,
        visibility: collectionVisibility(),
        questionIds: current.questionIds,
        revision: current.revision,
      });
      const saved = savedResult.value;
      setDraft({
        ...current,
        reference: saved.reference,
        revision: savedResult.etag,
        title: saved.title,
        visibility: saved.visibility,
      });
      setCollections((page) => {
        const existing = page.items.filter((item) => item.reference !== saved.reference);
        return pageWith([...existing, saved], page.nextCursor);
      });
      setShowCollectionForm(false);
      setNotice({
        kind: "success",
        text: `${saved.title} now contains ${current.questionIds.length} ordered questions.`,
      });
    } catch (error: unknown) {
      setNotice(
        failureNotice(error, "The collection could not save. Your ordered draft is retained."),
      );
    }
  }

  function useSelection(selection: ProblemPickerSelection): void {
    setStagedQuestionIds(selection.questionIds);
    setShowPicker(false);
    setNotice({
      kind: "success",
      text: `${selection.questionIds.length} selected questions are ready. Choose Favorites or a collection destination.`,
    });
  }

  function closePickerBrowser(): void {
    setShowPicker(false);
  }

  async function addStagedTo(collection: ProblemCollectionSummaryView): Promise<void> {
    if (!(await loadCollection(collection))) return;
    setDraft((current) => ({
      ...current,
      questionIds: appendCollectionQuestionIds(current.questionIds, stagedQuestionIds()),
    }));
    setCollectionTitle(collection.title);
    setCollectionVisibility(collection.visibility);
    setShowCollectionForm(true);
    setNotice({
      kind: "success",
      text: `Review the ordered ${collection.title} list, then save the complete update.`,
    });
  }

  async function saveCurrentSearch(): Promise<void> {
    const title = savedSearchTitle().trim();
    if (title.length === 0) {
      setNotice({ kind: "error", text: "Name this search before saving it." });
      return;
    }
    setNotice({ kind: "working", text: `Saving ${title}.` });
    try {
      const current = editingSavedSearch();
      const savedResult = await props.repository.replaceSavedSearch(
        savedSearchReplacementFromObserved(current, title, props.query()),
      );
      const saved = savedResult.value;
      setSavedSearches((page) =>
        pageWith(
          [...page.items.filter((item) => item.reference !== saved.reference), saved],
          page.nextCursor,
        ),
      );
      setSavedSearchTitle("");
      setEditingSavedSearch(undefined);
      setShowSavedSearchForm(false);
      setNotice({
        kind: "success",
        text: `${saved.title} now reruns this search against the current catalog.`,
      });
    } catch (error: unknown) {
      setNotice(
        failureNotice(
          error,
          "That search could not save. Keep its name and try again.",
          "Someone changed this saved search. Reload curation, review the current search, then save your retained name again.",
        ),
      );
    }
  }

  function requestDeletion(deletion: ProblemCurationDeletion, trigger: HTMLButtonElement): void {
    setDeletionBusy(false);
    setPendingDeletion({ deletion, trigger });
  }

  function closeDeletionConfirmation(returnFocus: boolean): void {
    const pending = pendingDeletion();
    setPendingDeletion(undefined);
    setDeletionBusy(false);
    if (returnFocus) queueMicrotask(() => pending?.trigger.focus({ preventScroll: true }));
  }

  async function confirmDeletion(): Promise<void> {
    const pending = pendingDeletion();
    if (pending === undefined || deletionBusy()) return;
    const deletion = pending.deletion;
    setDeletionBusy(true);
    setNotice({ kind: "working", text: `Deleting ${deletion.title}.` });
    try {
      if (deletion.kind === "collection") {
        await props.repository.deleteCollection(deletion.reference, deletion.revision);
        setCollections((page) =>
          pageWith(
            page.items.filter((item) => item.reference !== deletion.reference),
            page.nextCursor,
          ),
        );
        if (draft().reference === deletion.reference) setDraft(EMPTY_COLLECTION_DRAFT);
      } else {
        await props.repository.deleteSavedSearch(deletion.reference, deletion.revision);
        setSavedSearches((page) =>
          pageWith(
            page.items.filter((item) => item.reference !== deletion.reference),
            page.nextCursor,
          ),
        );
      }
      closeDeletionConfirmation(false);
      setNotice({ kind: "success", text: `${deletion.title} was deleted.` });
      queueMicrotask(() =>
        (deletion.kind === "collection" ? collectionsHeading : savedSearchesHeading)?.focus({
          preventScroll: true,
        }),
      );
    } catch (error: unknown) {
      closeDeletionConfirmation(true);
      setNotice(
        failureNotice(
          error,
          `${deletion.title} could not be deleted. Reload curation and choose Delete again.`,
          `Someone changed ${deletion.title}. Reload curation, review the current item, then choose Delete again.`,
        ),
      );
    }
  }

  onMount(() => {
    if (props.mayMutatePersonalCuration) void ensureFavorites();
    void loadCollections(null, false);
    if (props.mayMutatePersonalCuration) void loadSavedSearches(null, false);
  });

  return (
    <section class="problem-curation-panel" aria-labelledby="curation-heading">
      <header>
        <p class="eyebrow">Reuse workspace</p>
        <h2 id="curation-heading">Organize questions for teaching</h2>
        <p>
          Save useful questions and searches here. Each saved list retains the order you choose.
        </p>
      </header>
      <p class={`problem-curation-notice ${notice().kind}`} role="status" aria-live="polite">
        {noticeText(notice())}
      </p>
      <Show when={notice().kind === "error"}>
        <button class="quiet-action" type="button" onClick={() => void reloadCuration()}>
          Reload curation
        </button>
      </Show>
      <div class="problem-curation-actions">
        <button
          ref={(element) => {
            pickerTrigger = element;
          }}
          class="primary-action"
          type="button"
          onClick={() => setShowPicker(true)}
        >
          {props.mayMutatePersonalCuration ? "Select questions" : "Browse reusable questions"}
        </button>
        <Show when={props.mayMutatePersonalCuration}>
          <button class="quiet-action" type="button" onClick={() => setShowSavedSearchForm(true)}>
            Save current search
          </button>
          <button
            class="quiet-action"
            type="button"
            onClick={() => {
              setDraft(EMPTY_COLLECTION_DRAFT);
              setCanEditDraft(true);
              setCollectionTitle("");
              setCollectionVisibility("private");
              setShowCollectionForm(true);
            }}
          >
            Create collection
          </button>
        </Show>
      </div>

      <Show when={props.mayMutatePersonalCuration && stagedQuestionIds().length > 0}>
        <section class="problem-curation-staged" aria-labelledby="staged-selection-heading">
          <h3 id="staged-selection-heading">Selected questions ready to save</h3>
          <p>{stagedQuestionIds().length} questions will be appended in this order.</p>
          <div class="problem-curation-actions">
            <Show when={favorite()}>
              {(favorites) => (
                <button
                  class="primary-action"
                  type="button"
                  onClick={() => void addStagedTo(favorites())}
                >
                  Add to Favorites
                </button>
              )}
            </Show>
            <Show when={!favorite()}>
              <p>Favorites is preparing your personal collection.</p>
            </Show>
            <For
              each={collections().items.filter(
                (collection) => collection.kind === "named" && collection.access === "owner",
              )}
            >
              {(collection) => (
                <button
                  class="quiet-action"
                  type="button"
                  onClick={() => void addStagedTo(collection)}
                >
                  Add to {collection.title}
                </button>
              )}
            </For>
          </div>
        </section>
      </Show>

      <Show when={showCollectionForm() && props.mayMutatePersonalCuration && canEditDraft()}>
        <section class="problem-curation-editor" aria-labelledby="collection-editor-heading">
          <h3 id="collection-editor-heading">
            {draft().reference === null ? "Create collection" : "Update collection"}
          </h3>
          <label>
            Collection name
            <input
              value={collectionTitle()}
              maxlength={200}
              disabled={!canEditDraft()}
              onInput={(event) => setCollectionTitle(event.currentTarget.value)}
            />
          </label>
          <label>
            Visibility
            <select
              value={collectionVisibility()}
              disabled={!canEditDraft()}
              onChange={(event) =>
                setCollectionVisibility(
                  event.currentTarget.value === "institution" ? "institution" : "private",
                )
              }
            >
              <option value="private">Private to me</option>
              <option value="institution">Share with my institution</option>
            </select>
          </label>
          <p>{draft().questionIds.length} questions in this ordered collection.</p>
          <Show when={!canEditDraft()}>
            <p>
              This institution collection is ready to browse and reuse. Its owner controls changes.
            </p>
          </Show>
          <ol class="problem-curation-members">
            <For each={draft().questionIds}>
              {(questionId, index) => (
                <li>
                  <span>{questionId}</span>
                  <span class="problem-curation-member-actions">
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={!canEditDraft() || index() === 0}
                      onClick={() =>
                        setDraft((current) => ({
                          ...current,
                          questionIds: moveCollectionQuestionId(current.questionIds, index(), -1),
                        }))
                      }
                    >
                      Earlier
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={!canEditDraft() || index() === draft().questionIds.length - 1}
                      onClick={() =>
                        setDraft((current) => ({
                          ...current,
                          questionIds: moveCollectionQuestionId(current.questionIds, index(), 1),
                        }))
                      }
                    >
                      Later
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() =>
                        setDraft((current) => ({
                          ...current,
                          questionIds: removeCollectionQuestionId(current.questionIds, questionId),
                        }))
                      }
                    >
                      Remove
                    </button>
                  </span>
                </li>
              )}
            </For>
          </ol>
          <div class="problem-curation-actions">
            <button
              class="primary-action"
              type="button"
              disabled={!canEditDraft()}
              onClick={() => void saveDraft()}
            >
              Save collection
            </button>
            <button class="quiet-action" type="button" onClick={() => setShowCollectionForm(false)}>
              Keep editing later
            </button>
          </div>
        </section>
      </Show>

      <Show when={openedCollection() !== undefined && !canEditDraft()}>
        <section class="problem-curation-detail" aria-labelledby="collection-detail-heading">
          <p class="eyebrow">Institution collection</p>
          <h3 id="collection-detail-heading">{openedCollection()!.title}</h3>
          <p>
            {openedCollection()!.visibility === "institution"
              ? "Shared with this institution"
              : "Private collection"}{" "}
            · revision {openedCollection()!.revision} · access: {openedCollection()!.access}
          </p>
          <p>
            Browse these current published questions or reuse them through a question picker. The
            collection owner controls its name, membership, order, and sharing.
          </p>
          <ol class="problem-curation-members">
            <For each={openedMembers().items}>
              {(member) => (
                <li>
                  <span>
                    <strong>{member.summary.metadata.title}</strong> · {member.questionId}
                  </span>
                  <span>
                    {member.selectionAvailability === "available"
                      ? "Ready to reuse"
                      : "Retained record"}
                  </span>
                </li>
              )}
            </For>
          </ol>
          <Show when={openedMembers().nextCursor !== null}>
            <button class="quiet-action" type="button" onClick={() => void loadMoreOpenedMembers()}>
              Load more collection members
            </button>
          </Show>
        </section>
      </Show>

      <Show when={showSavedSearchForm() && props.mayMutatePersonalCuration}>
        <section class="problem-curation-editor" aria-labelledby="saved-search-heading">
          <h3 id="saved-search-heading">
            {editingSavedSearch() === undefined
              ? "Save this current search"
              : "Rename saved search"}
          </h3>
          <p>The saved search runs against current published questions each time you use it.</p>
          <label>
            Search name
            <input
              value={savedSearchTitle()}
              maxlength={200}
              onInput={(event) => setSavedSearchTitle(event.currentTarget.value)}
            />
          </label>
          <div class="problem-curation-actions">
            <button class="primary-action" type="button" onClick={() => void saveCurrentSearch()}>
              Save search
            </button>
            <button
              class="quiet-action"
              type="button"
              onClick={() => setShowSavedSearchForm(false)}
            >
              Keep editing filters
            </button>
          </div>
        </section>
      </Show>

      <section class="problem-curation-list" aria-labelledby="collections-heading">
        <h3
          id="collections-heading"
          tabindex={-1}
          ref={(element) => {
            collectionsHeading = element;
          }}
        >
          Collections
        </h3>
        <Show
          when={collections().items.length > 0}
          fallback={<p>Create a collection or select questions to begin.</p>}
        >
          <ul>
            <For each={collections().items}>
              {(collection) => (
                <li>
                  <div>
                    <strong>{collection.title}</strong>
                    <span>
                      {collection.kind === "favorites"
                        ? "Favorites"
                        : collection.visibility === "institution"
                          ? "Institution collection"
                          : "Private collection"}
                    </span>
                  </div>
                  <div class="problem-curation-actions">
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() => void loadCollection(collection)}
                    >
                      Open
                    </button>
                    <Show when={collection.access === "owner" && collection.kind === "named"}>
                      <button
                        class="quiet-action"
                        type="button"
                        aria-label={`Delete collection ${collection.title}`}
                        onClick={(event) =>
                          requestDeletion(
                            collectionDeletionFromObserved(collection),
                            event.currentTarget,
                          )
                        }
                      >
                        Delete
                      </button>
                    </Show>
                  </div>
                </li>
              )}
            </For>
          </ul>
          <Show when={collections().nextCursor !== null}>
            <button
              class="quiet-action"
              type="button"
              onClick={() => void loadCollections(collections().nextCursor, true)}
            >
              Load more collections
            </button>
          </Show>
        </Show>
      </section>

      <Show when={props.mayMutatePersonalCuration}>
        <section class="problem-curation-list" aria-labelledby="saved-searches-heading">
          <h3
            id="saved-searches-heading"
            tabindex={-1}
            ref={(element) => {
              savedSearchesHeading = element;
            }}
          >
            Saved searches
          </h3>
          <Show
            when={savedSearches().items.length > 0}
            fallback={<p>Save the current discovery filters to reuse them later.</p>}
          >
            <ul>
              <For each={savedSearches().items}>
                {(search) => (
                  <li>
                    <div>
                      <strong>{search.title}</strong>
                      <span>Runs current catalog results</span>
                    </div>
                    <div class="problem-curation-actions">
                      <button
                        class="quiet-action"
                        type="button"
                        onClick={() => {
                          props.applyQuery(libraryQueryFromSavedSearch(search));
                          setNotice({
                            kind: "success",
                            text: `${search.title} is running against the current catalog.`,
                          });
                        }}
                      >
                        Run search
                      </button>
                      <button
                        class="quiet-action"
                        type="button"
                        onClick={() => {
                          setEditingSavedSearch(search);
                          setSavedSearchTitle(search.title);
                          setShowSavedSearchForm(true);
                        }}
                      >
                        Rename
                      </button>
                      <button
                        class="quiet-action"
                        type="button"
                        aria-label={`Delete saved search ${search.title}`}
                        onClick={(event) =>
                          requestDeletion(
                            savedSearchDeletionFromObserved(search),
                            event.currentTarget,
                          )
                        }
                      >
                        Delete
                      </button>
                    </div>
                  </li>
                )}
              </For>
            </ul>
            <Show when={savedSearches().nextCursor !== null}>
              <button
                class="quiet-action"
                type="button"
                onClick={() => void loadSavedSearches(savedSearches().nextCursor, true)}
              >
                Load more searches
              </button>
            </Show>
          </Show>
        </section>
      </Show>

      <Show when={showPicker()}>
        <ProblemPicker
          repository={props.pickerRepository}
          sources={problemCurationPickerSources(
            collections().items,
            props.mayMutatePersonalCuration,
          )}
          mode={props.mayMutatePersonalCuration ? "many" : "none"}
          maximumSelection={MAX_PROBLEM_COLLECTION_MEMBERS}
          trigger={pickerTrigger}
          title="Select published questions"
          confirmLabel={
            props.mayMutatePersonalCuration ? "Prepare selected questions" : "Close browser"
          }
          onConfirm={props.mayMutatePersonalCuration ? useSelection : closePickerBrowser}
          onCancel={closePickerBrowser}
        />
      </Show>

      <Show when={pendingDeletion()}>
        {(pending) => (
          <ProblemCurationConfirmationDialog
            deletion={pending().deletion}
            busy={deletionBusy()}
            onCancel={() => closeDeletionConfirmation(true)}
            onConfirm={() => void confirmDeletion()}
          />
        )}
      </Show>
    </section>
  );
}
