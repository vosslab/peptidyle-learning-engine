// question_curation_panel.tsx - live private Question Folder and Saved Question Search workspace.

import { For, Show, createSignal, onMount, type Accessor, type JSX } from "solid-js";

import type { QuestionFolderEntryView } from "../../../generated/api/QuestionFolderEntryView";
import { QuestionCurationConflictError } from "../../api/http_client";
import type { QuestionFolderSummaryView } from "../../../generated/api/QuestionFolderSummaryView";
import type { SavedQuestionSearchView } from "../../../generated/api/SavedQuestionSearchView";
import { MAX_QUESTION_FOLDER_MEMBERS } from "../../../generated/api/MAX_QUESTION_FOLDER_MEMBERS";
import {
  QuestionPicker,
  type QuestionPickerSelection,
  type QuestionPickerSourceRepository,
} from "../question_picker";
import type { QuestionSearchQuery } from "../../pages/library_page_model";
import "./question_curation.css";
import { QuestionCurationConfirmationDialog } from "./question_curation_confirmation";
import {
  EMPTY_FOLDER_DRAFT,
  appendFolderQuestionIds,
  folderDraftFrom,
  folderDeletionFromObserved,
  libraryQueryFromSavedSearch,
  mayEditOpenedQuestionFolder,
  moveFolderQuestionId,
  questionCurationPickerSources,
  removeFolderQuestionId,
  savedSearchDeletionFromObserved,
  savedSearchReplacementFromObserved,
  type FolderDraft,
  type CurationNotice,
  type QuestionCurationDeletion,
  type QuestionCurationPage,
  type QuestionCurationRepository,
} from "./question_curation_model";

export interface QuestionCurationPanelProps {
  readonly repository: QuestionCurationRepository;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly query: Accessor<QuestionSearchQuery>;
  readonly applyQuery: (query: QuestionSearchQuery) => void;
  /** Instructor authority enables private Question Folders and Saved Question Searches. */
  readonly mayMutatePersonalCuration: boolean;
}

function noticeText(notice: CurationNotice): string {
  return notice.kind === "idle"
    ? "Choose a visible action to organize questions you can reuse."
    : notice.text;
}

function failureNotice(error: unknown, ordinary: string, conflict?: string): CurationNotice {
  if (error instanceof QuestionCurationConflictError) {
    return {
      kind: "error",
      text:
        conflict ??
        "Someone saved a newer version first. Reload curation, review the current version, then apply your retained draft.",
    };
  }
  return { kind: "error", text: ordinary };
}

function pageWith<T>(items: ReadonlyArray<T>, nextCursor: string | null): QuestionCurationPage<T> {
  return { items, nextCursor };
}

/**
 * Visible task-order curation: discover, stage a bounded selection, choose a
 * named destination, then receive a durable result or a clear recovery action.
 */
export function QuestionCurationPanel(props: QuestionCurationPanelProps): JSX.Element {
  type PendingDeletion = {
    readonly deletion: QuestionCurationDeletion;
    readonly trigger: HTMLButtonElement;
  };

  const [collections, setCollections] = createSignal<
    QuestionCurationPage<QuestionFolderSummaryView>
  >(pageWith([], null));
  const [savedSearches, setSavedSearches] = createSignal<
    QuestionCurationPage<SavedQuestionSearchView>
  >(pageWith([], null));
  const [draft, setDraft] = createSignal<FolderDraft>(EMPTY_FOLDER_DRAFT);
  const [stagedQuestionIds, setStagedQuestionIds] = createSignal<ReadonlyArray<string>>([]);
  const [notice, setNotice] = createSignal<CurationNotice>({ kind: "idle" });
  const [showPicker, setShowPicker] = createSignal(false);
  const [showCollectionForm, setShowCollectionForm] = createSignal(false);
  const [showSavedSearchForm, setShowSavedSearchForm] = createSignal(false);
  const [canEditDraft, setCanEditDraft] = createSignal(true);
  const [openedCollection, setOpenedCollection] = createSignal<QuestionFolderSummaryView>();
  const [openedMembers, setOpenedMembers] = createSignal<
    QuestionCurationPage<QuestionFolderEntryView>
  >(pageWith([], null));
  const [collectionTitle, setCollectionTitle] = createSignal("");
  const [savedSearchTitle, setSavedSearchTitle] = createSignal("");
  const [editingSavedSearch, setEditingSavedSearch] = createSignal<
    SavedQuestionSearchView | undefined
  >();
  const [pendingDeletion, setPendingDeletion] = createSignal<PendingDeletion>();
  const [deletionBusy, setDeletionBusy] = createSignal(false);
  let pickerTrigger: HTMLButtonElement | undefined;
  let collectionsHeading: HTMLHeadingElement | undefined;
  let savedSearchesHeading: HTMLHeadingElement | undefined;

  async function loadCollections(cursor: string | null, append: boolean): Promise<boolean> {
    try {
      const page = await props.repository.listFolders(cursor);
      setCollections((current) =>
        append ? pageWith([...current.items, ...page.items], page.nextCursor) : page,
      );
      return true;
    } catch (error: unknown) {
      setNotice(failureNotice(error, "Question Folders could not load. Try loading them again."));
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
      ...(props.mayMutatePersonalCuration ? [loadSavedSearches(null, false)] : []),
    ]);
    if (results.some((loaded) => !loaded)) return;

    const opened = openedCollection();
    if (opened !== undefined && canEditDraft()) {
      try {
        const current = await props.repository.getFolder(opened.reference);
        setOpenedCollection(current.value);
        setDraft((local) =>
          local.reference === opened.reference ? { ...local, editNumber: current.etag } : local,
        );
        setNotice({
          kind: "success",
          text: `${current.value.title} is current. Your selected questions and edits are ready to review and save.`,
        });
        return;
      } catch (error: unknown) {
        setNotice(failureNotice(error, "The current Question Folder could not reload. Try again."));
        return;
      }
    }
    setNotice({ kind: "success", text: "Current curation is loaded." });
  }

  async function loadCollection(collection: QuestionFolderSummaryView): Promise<boolean> {
    setNotice({ kind: "working", text: `Loading Question Folder ${collection.title}.` });
    try {
      const current = await props.repository.getFolder(collection.reference);
      setOpenedCollection(current.value);
      const mayEdit = mayEditOpenedQuestionFolder(props.mayMutatePersonalCuration);
      setCanEditDraft(mayEdit);
      if (!mayEdit) {
        const page = await props.repository.listFolderEntries(collection.reference, null);
        setOpenedMembers(pageWith(page.items, page.nextCursor));
        setNotice({
          kind: "success",
          text: `${current.value.title} is available to browse and reuse. Its owner controls changes.`,
        });
        return true;
      }
      const loaded: QuestionFolderEntryView[] = [];
      let etag = current.etag;
      let cursor: string | null = null;
      do {
        const page = await props.repository.listFolderEntries(collection.reference, cursor);
        loaded.push(...page.items);
        etag = page.etag;
        cursor = page.nextCursor;
      } while (cursor !== null && loaded.length < MAX_QUESTION_FOLDER_MEMBERS);
      setDraft({
        ...folderDraftFrom(current.value, loaded),
        editNumber: etag,
      });
      setCollectionTitle(current.value.title);
      setCanEditDraft(mayEdit);
      setOpenedMembers(pageWith(loaded, null));
      setShowCollectionForm(true);
      setNotice({
        kind: "success",
        text: `${current.value.title} is ready to edit.`,
      });
      return true;
    } catch (error: unknown) {
      setNotice(
        failureNotice(
          error,
          "That Question Folder could not load. Your current selection is still available.",
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
      const page = await props.repository.listFolderEntries(collection.reference, cursor);
      setOpenedMembers((current) => pageWith([...current.items, ...page.items], page.nextCursor));
    } catch (error: unknown) {
      setNotice(failureNotice(error, "More Question Folder entries could not load. Try again."));
    }
  }

  async function saveDraft(): Promise<void> {
    const current = draft();
    const title = collectionTitle().trim();
    if (title.length === 0) {
      setNotice({ kind: "error", text: "Name the Question Folder before saving it." });
      return;
    }
    setNotice({ kind: "working", text: `Saving ${title}.` });
    try {
      const savedResult = await props.repository.replaceFolder({
        reference: current.reference,
        title,
        questionIds: current.questionIds,
        editNumber: current.editNumber,
      });
      const saved = savedResult.value;
      setDraft({
        ...current,
        reference: saved.reference,
        editNumber: savedResult.etag,
        title: saved.title,
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
        failureNotice(error, "The Question Folder could not save. Your ordered draft is retained."),
      );
    }
  }

  function useSelection(selection: QuestionPickerSelection): void {
    setStagedQuestionIds(selection.questionIds);
    setShowPicker(false);
    setNotice({
      kind: "success",
      text: `${selection.questionIds.length} selected Questions are ready. Choose a Question Folder destination.`,
    });
  }

  function closePickerBrowser(): void {
    setShowPicker(false);
  }

  async function addStagedTo(collection: QuestionFolderSummaryView): Promise<void> {
    if (!(await loadCollection(collection))) return;
    setDraft((current) => ({
      ...current,
      questionIds: appendFolderQuestionIds(current.questionIds, stagedQuestionIds()),
    }));
    setCollectionTitle(collection.title);
    setShowCollectionForm(true);
    setNotice({
      kind: "success",
      text: `Review the ordered Question Folder ${collection.title}, then save the complete update.`,
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
        text: `${saved.title} now reruns this search against the current Question Library.`,
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

  function requestDeletion(deletion: QuestionCurationDeletion, trigger: HTMLButtonElement): void {
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
      if (deletion.kind === "folder") {
        await props.repository.deleteFolder(deletion.reference, deletion.editNumber);
        setCollections((page) =>
          pageWith(
            page.items.filter((item) => item.reference !== deletion.reference),
            page.nextCursor,
          ),
        );
        if (draft().reference === deletion.reference) setDraft(EMPTY_FOLDER_DRAFT);
      } else {
        await props.repository.deleteSavedSearch(deletion.reference, deletion.editNumber);
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
        (deletion.kind === "folder" ? collectionsHeading : savedSearchesHeading)?.focus({
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
    void loadCollections(null, false);
    if (props.mayMutatePersonalCuration) void loadSavedSearches(null, false);
  });

  return (
    <section class="question-curation-panel" aria-labelledby="curation-heading">
      <header>
        <p class="eyebrow">Reuse workspace</p>
        <h2 id="curation-heading">Organize questions for teaching</h2>
        <p>
          Save useful questions and searches here. Each saved list retains the order you choose.
        </p>
      </header>
      <p class={`question-curation-notice ${notice().kind}`} role="status" aria-live="polite">
        {noticeText(notice())}
      </p>
      <Show when={notice().kind === "error"}>
        <button class="quiet-action" type="button" onClick={() => void reloadCuration()}>
          Reload curation
        </button>
      </Show>
      <div class="question-curation-actions">
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
              setDraft(EMPTY_FOLDER_DRAFT);
              setCanEditDraft(true);
              setCollectionTitle("");
              setShowCollectionForm(true);
            }}
          >
            Create Question Folder
          </button>
        </Show>
      </div>

      <Show when={props.mayMutatePersonalCuration && stagedQuestionIds().length > 0}>
        <section class="question-curation-staged" aria-labelledby="staged-selection-heading">
          <h3 id="staged-selection-heading">Selected questions ready to save</h3>
          <p>{stagedQuestionIds().length} questions will be appended in this order.</p>
          <div class="question-curation-actions">
            <For each={collections().items}>
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
        <section class="question-curation-editor" aria-labelledby="collection-editor-heading">
          <h3 id="collection-editor-heading">
            {draft().reference === null ? "Create Question Folder" : "Update Question Folder"}
          </h3>
          <label>
            Question Folder name
            <input
              value={collectionTitle()}
              maxlength={200}
              disabled={!canEditDraft()}
              onInput={(event) => setCollectionTitle(event.currentTarget.value)}
            />
          </label>
          <p>{draft().questionIds.length} Questions in this ordered Question Folder.</p>
          <ol class="question-curation-members">
            <For each={draft().questionIds}>
              {(questionId, index) => (
                <li>
                  <span>{questionId}</span>
                  <span class="question-curation-member-actions">
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={!canEditDraft() || index() === 0}
                      onClick={() =>
                        setDraft((current) => ({
                          ...current,
                          questionIds: moveFolderQuestionId(current.questionIds, index(), -1),
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
                          questionIds: moveFolderQuestionId(current.questionIds, index(), 1),
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
                          questionIds: removeFolderQuestionId(current.questionIds, questionId),
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
          <div class="question-curation-actions">
            <button
              class="primary-action"
              type="button"
              disabled={!canEditDraft()}
              onClick={() => void saveDraft()}
            >
              Save Question Folder
            </button>
            <button class="quiet-action" type="button" onClick={() => setShowCollectionForm(false)}>
              Keep editing later
            </button>
          </div>
        </section>
      </Show>

      <Show when={openedCollection() !== undefined && !canEditDraft()}>
        <section class="question-curation-detail" aria-labelledby="collection-detail-heading">
          <p class="eyebrow">Private Question Folder</p>
          <h3 id="collection-detail-heading">{openedCollection()!.title}</h3>
          <p>Private Question Folder - edit number {openedCollection()!.editNumber}</p>
          <p>Browse these current published questions or reuse them through a question picker.</p>
          <ol class="question-curation-members">
            <For each={openedMembers().items}>
              {(member) => (
                <li>
                  <span>
                    <strong>{member.summary.metadata.title}</strong> - {member.questionId}
                  </span>
                  <span>
                    {member.questionVersionAvailability.availability === "available"
                      ? "Ready to reuse"
                      : "Archived version"}
                  </span>
                </li>
              )}
            </For>
          </ol>
          <Show when={openedMembers().nextCursor !== null}>
            <button class="quiet-action" type="button" onClick={() => void loadMoreOpenedMembers()}>
              Load more Question Folder Entries
            </button>
          </Show>
        </section>
      </Show>

      <Show when={showSavedSearchForm() && props.mayMutatePersonalCuration}>
        <section class="question-curation-editor" aria-labelledby="saved-search-heading">
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
          <div class="question-curation-actions">
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

      <section class="question-curation-list" aria-labelledby="collections-heading">
        <h3
          id="collections-heading"
          tabindex={-1}
          ref={(element) => {
            collectionsHeading = element;
          }}
        >
          Question Folders
        </h3>
        <Show
          when={collections().items.length > 0}
          fallback={<p>Create a Question Folder or select Questions to begin.</p>}
        >
          <ul>
            <For each={collections().items}>
              {(collection) => (
                <li>
                  <div>
                    <strong>{collection.title}</strong>
                    <span>Private Question Folder</span>
                  </div>
                  <div class="question-curation-actions">
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() => void loadCollection(collection)}
                    >
                      Open
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      aria-label={`Delete Question Folder ${collection.title}`}
                      onClick={(event) =>
                        requestDeletion(folderDeletionFromObserved(collection), event.currentTarget)
                      }
                    >
                      Delete
                    </button>
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
              Load more Question Folders
            </button>
          </Show>
        </Show>
      </section>

      <Show when={props.mayMutatePersonalCuration}>
        <section class="question-curation-list" aria-labelledby="saved-searches-heading">
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
                      <span>Runs current Question Library results</span>
                    </div>
                    <div class="question-curation-actions">
                      <button
                        class="quiet-action"
                        type="button"
                        onClick={() => {
                          props.applyQuery(libraryQueryFromSavedSearch(search));
                          setNotice({
                            kind: "success",
                            text: `${search.title} is running against the current Question Library.`,
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
        <QuestionPicker
          repository={props.pickerRepository}
          sources={questionCurationPickerSources(
            collections().items,
            props.mayMutatePersonalCuration,
          )}
          mode={props.mayMutatePersonalCuration ? "many" : "none"}
          maximumSelection={MAX_QUESTION_FOLDER_MEMBERS}
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
          <QuestionCurationConfirmationDialog
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
