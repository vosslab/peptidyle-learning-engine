// library_page.tsx - injected Question Library browse surface; route wiring follows the server contract.

import { useLocation, useNavigate } from "@solidjs/router";
import { Show, createEffect, createSignal, onCleanup, type JSX } from "solid-js";

import { createLibraryBulkActions, LibraryBulkActions } from "./library_bulk_actions";
import { QuestionPoolCreateDialog } from "../components/question_pool_create_dialog";
import type { QuestionPoolStartingQuestion } from "../components/question_pool_create_model";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionPickerProps } from "../features/question_picker";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { QuestionPoolSearchMetadataClient } from "../api/question_pool_search_metadata";
import type { QuestionPoolCreationClient } from "../api/question_pool_creation";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame/page_frame";
import { confirmUnsavedChanges } from "../components/unsaved_changes_guard";
import "./library_page.css";
import { createSearchState } from "../components/search_block/search_state";
import { SearchControls } from "../components/search_block/search_controls";
import { SearchBlock } from "../components/search_block/search_block";
import { fixedKindLibrarySearchFields, librarySearchFields } from "./library_search_fields";
import { libraryRecordQuery } from "./library_record_filters";
import {
  hasLibrarySearchCriteria,
  questionLibrarySearchDefinition,
} from "./question_library_search_definition";
import { LibraryBrowseControls } from "./library_browse_controls";
import { LibrarySearchFilters } from "./library_search_filters";
import {
  searchHandoffQuery,
  searchWithinResultsPath,
  recoverLibrarySearch,
} from "./library_search_parameters";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowseQuery,
} from "./library_page_model";

export interface LibraryPageProps {
  readonly mode: "search" | "browse";
  readonly collectionScope?: "myQuestions" | "myPools";
  readonly repository: QuestionLibraryBrowseRepository;
  readonly metadataClient: QuestionBulkMetadataClient;
  readonly poolMetadataClient: QuestionPoolSearchMetadataClient;
  readonly classificationClient: import("../api/content_classification").ContentClassificationClient;
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>;
  readonly loadQuestionInspection: QuestionPickerProps["loadQuestionInspection"];
  readonly questionRevisionPreviewDocumentUrl: QuestionPickerProps["questionRevisionPreviewDocumentUrl"];
  readonly questionImageUrl: QuestionPickerProps["questionImageUrl"];
}

/** Question Library UI with the production repository injected by the route composition. */
export function LibraryPage(props: LibraryPageProps): JSX.Element {
  const myQuestions = props.collectionScope === "myQuestions";
  const myPools = props.collectionScope === "myPools";
  const personalCollection = myQuestions || myPools;
  const sessionBootstrapState = useSessionBootstrap().state();
  if (sessionBootstrapState.kind !== "authenticated") {
    throw new Error("Question Library requires an authenticated session scope");
  }
  const sessionScope = sessionBootstrapState.session;
  const initialQuery: QuestionLibraryBrowseQuery = myQuestions
    ? { ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY, kind: "questions", questions: "all" }
    : myPools
      ? {
          ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
          kind: "pools",
          questions: "all",
          ownerAccountId: sessionScope.account.id,
        }
      : EMPTY_QUESTION_LIBRARY_BROWSE_QUERY;
  const mayMutateLibrary = sessionScope.account.userRole === "instructor";
  const location = useLocation();
  const navigate = useNavigate();
  // Catch only URL parsing at the route boundary; transport validation remains strict.
  function routeHandoff(search: string): QuestionLibraryBrowseQuery | null {
    try {
      const parsed = searchHandoffQuery(search);
      if (myPools) {
        // ASVS 2.2.1: parse URL options before fixing the authenticated Pool scope.
        return {
          ...parsed,
          kind: "pools",
          questions: "all",
          ownerAccountId: sessionScope.account.id,
        };
      }
      return myQuestions
        ? {
            ...parsed,
            kind: "questions",
            questions: new URLSearchParams(search).has("questions") ? parsed.questions : "all",
          }
        : parsed;
    } catch {
      return null;
    }
  }
  const initialHandoffQuery = routeHandoff(location.search);
  const [invalidLinkOptions, setInvalidLinkOptions] = createSignal(initialHandoffQuery === null);
  let disposed = false;
  const metadataGuardScope = Symbol("Library metadata editors");
  onCleanup(() => {
    disposed = true;
  });
  const searchState = createSearchState(
    questionLibrarySearchDefinition(
      props.repository,
      initialQuery,
      personalCollection,
      myQuestions ? "questions" : myPools ? "pools" : undefined,
      myPools ? sessionScope.account.id : undefined,
    ),
    {
      initialQuery: initialHandoffQuery ?? initialQuery,
      beforeQueryChange: async (): Promise<boolean> => {
        if (!(await confirmUnsavedChanges(metadataGuardScope)) || disposed) return false;
        bulkActions.clearForQueryChange();
        return true;
      },
    },
  );
  const query = searchState.query;
  const [recordFilterError, setRecordFilterError] = createSignal<string | null>(null);
  const recordQuery = libraryRecordQuery({
    query,
    apply: searchState.apply,
    sort: searchState.sort,
    replacementRevision: searchState.replacementRevision,
    repository: props.repository,
    classificationClient: props.classificationClient,
    onError: setRecordFilterError,
  });
  const state = searchState.state;
  const selectedIds = searchState.selectedIds;
  const [questionPoolCreateOpen, setQuestionPoolCreateOpen] = createSignal(false);
  const [questionPoolTaskActive, setQuestionPoolTaskActive] = createSignal(false);
  const [createStartingQuestions, setCreateStartingQuestions] = createSignal<
    ReadonlyArray<QuestionPoolStartingQuestion>
  >([]);
  createEffect(() => {
    const routeSearch = location.search;
    const handoffQuery = routeHandoff(routeSearch);
    setInvalidLinkOptions(handoffQuery === null);
    if (handoffQuery === null) return;
    if (personalCollection || props.mode === "browse" || routeSearch.length > 0)
      void searchState.open(handoffQuery);
  });

  const bulkActions = createLibraryBulkActions({
    state: searchState,
    displayedRows: () => state().rows,
    metadataClient: props.metadataClient,
    onCreatePoolFromQuestions: (startingQuestions): void => {
      setCreateStartingQuestions(startingQuestions);
      setQuestionPoolCreateOpen(true);
    },
    guardScope: metadataGuardScope,
    refresh: searchState.refresh,
  });
  const editorBusy = bulkActions.busy;
  function changeQuery(change: Partial<QuestionLibraryBrowseQuery>): void {
    if (invalidLinkOptions()) return;
    void searchState.apply((current) => ({
      ...current,
      ...change,
      ...(myQuestions ? { kind: "questions" as const } : {}),
      ...(myPools ? { kind: "pools" as const, ownerAccountId: sessionScope.account.id } : {}),
    }));
  }

  function startOverBrowse(): void {
    void searchState.open(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY);
  }

  function resetInvalidLinkOptions(): void {
    const search = recoverLibrarySearch(location.search);
    const recoveredQuery = routeHandoff(search);
    if (recoveredQuery === null) return;
    setInvalidLinkOptions(false);
    navigate(`${location.pathname}${search}${location.hash}`, { replace: true });
    if (personalCollection || props.mode === "browse" || search.length > 0) {
      void searchState.open(recoveredQuery);
    } else {
      void searchState.clear();
    }
  }

  function createQuestionPoolAction(): JSX.Element {
    return (
      <Show when={mayMutateLibrary}>
        <button
          type="button"
          class={props.mode === "browse" ? "primary-action" : undefined}
          disabled={editorBusy()}
          onClick={() => {
            setCreateStartingQuestions([]);
            setQuestionPoolCreateOpen(true);
          }}
        >
          Create Question Pool
        </button>
      </Show>
    );
  }

  return (
    <PageFrame
      // Library controls, browse groups, and the active pool-creation review.
      contentClass={`library-page${questionPoolTaskActive() ? " question-pool-task-active" : ""}`}
      routeSurface={
        myQuestions
          ? "myQuestions"
          : myPools
            ? "myPools"
            : props.mode === "browse"
              ? "library-browse"
              : "library"
      }
    >
      <SearchBlock
        state={searchState}
        layout={props.mode === "search" ? "above" : "beside"}
        controls={
          <>
            <Show when={invalidLinkOptions()}>
              <div role="alert">
                <p>
                  This Library link has invalid filter or order options. No search has been run.
                  Reset the invalid options to continue with the valid options in this link.
                </p>
                <button type="button" onClick={resetInvalidLinkOptions}>
                  Reset invalid Library options
                </button>
              </div>
            </Show>
            <Show when={!invalidLinkOptions()}>
              <div class="library-page__filters">
                <Show when={recordFilterError()}>
                  {(message) => <p role="alert">{message()}</p>}
                </Show>
                <Show when={props.mode === "browse" && mayMutateLibrary}>
                  <p>{createQuestionPoolAction()}</p>
                </Show>
                <p class="sr-only" role="status" aria-live="polite">
                  {state().kind === "loading" ? "Loading Question Library results." : ""}
                </p>
                <Show when={props.mode === "search"}>
                  <SearchControls
                    state={searchState}
                    textLabel={
                      myQuestions
                        ? "Search My Questions"
                        : myPools
                          ? "Search My Pools"
                          : "Search Question Library"
                    }
                    textPlaceholder="Title or concept"
                    disabled={editorBusy}
                  >
                    <div class="library-search-secondary-actions">
                      <LibrarySearchFilters
                        query={query}
                        disabled={editorBusy}
                        onChange={changeQuery}
                        fixedOwner={myPools}
                      />
                      <details class="question-library-search-tips">
                        <summary>Search tips</summary>
                        <div>
                          <p>
                            Ordinary words search together. Use quotes for a phrase and a leading
                            minus to exclude.
                          </p>
                          <p>
                            Fields: <code>discipline:</code>, <code>subject:</code>,{" "}
                            <code>topic:</code>, <code>subtopic:</code>, <code>tags:</code>,{" "}
                            <code>type:</code>, and <code>author:</code>.
                          </p>
                        </div>
                      </details>
                      {createQuestionPoolAction()}
                    </div>
                  </SearchControls>
                </Show>
                <Show when={props.mode === "browse"}>
                  <LibraryBrowseControls
                    query={query}
                    filters={recordQuery.filters!}
                    hasCriteria={() => hasLibrarySearchCriteria(query())}
                    searchWithinResultsPath={() => searchWithinResultsPath(query())}
                    disabled={editorBusy}
                    changeQuery={changeQuery}
                    startOver={startOverBrowse}
                  />
                </Show>
              </div>
            </Show>
          </>
        }
        collection={{
          ariaLabel: myQuestions
            ? "My Published Questions"
            : myPools
              ? "My Question Pools"
              : "Question Library results",
          fields: personalCollection ? fixedKindLibrarySearchFields : librarySearchFields,
          query: recordQuery,
          disabled: editorBusy,
          initialEmptyState: personalCollection
            ? undefined
            : {
                message:
                  props.mode === "browse"
                    ? "Choose a Discipline or use column filters to browse Library objects."
                    : "Enter a keyword or use the column filters to find Library objects.",
              },
          emptyState: {
            title: myQuestions
              ? "No Published Questions match these filters"
              : myPools
                ? "No Question Pools match these filters"
                : "No Library objects match these filters",
          },
          selection: () =>
            mayMutateLibrary
              ? {
                  kind: "checkbox" as const,
                  selectedIds,
                  disabled: () => editorBusy(),
                  onChange: (row, selected) => void bulkActions.updateSelection(row, selected),
                  toolbar: {
                    disabled: editorBusy,
                    onSelectLoaded: () => void bulkActions.selectLoaded(),
                    onClearSelection: () => void bulkActions.clearSelection(),
                    actions: bulkActions.toolbarActions,
                  },
                }
              : undefined,
        }}
      />
      <Show
        when={
          !invalidLinkOptions() &&
          (props.mode === "search" ||
            hasLibrarySearchCriteria(query()) ||
            state().kind === "error" ||
            editorBusy())
        }
      >
        <div class="library-page__results">
          <Show when={mayMutateLibrary}>
            <LibraryBulkActions
              controller={bulkActions}
              metadataClient={props.metadataClient}
              poolMetadataClient={props.poolMetadataClient}
              classificationClient={props.classificationClient}
            />
          </Show>
          <Show when={bulkActions.selectionNotice()}>
            {(notice) => <p role="status">{notice()}</p>}
          </Show>
        </div>
      </Show>
      <Show when={mayMutateLibrary && questionPoolCreateOpen()}>
        <div class="question-pool-create-host">
          <QuestionPoolCreateDialog
            questionPoolClient={props.questionPoolClient}
            questionLibrary={props.repository}
            getQuestionDetails={props.getQuestionDetails}
            loadQuestionInspection={props.loadQuestionInspection}
            questionRevisionPreviewDocumentUrl={props.questionRevisionPreviewDocumentUrl}
            questionImageUrl={props.questionImageUrl}
            startingQuestions={createStartingQuestions()}
            getCurrentQuestionSharedMetadata={props.metadataClient.getCurrentQuestionSharedMetadata}
            classificationClient={props.classificationClient}
            onTaskPhaseChange={setQuestionPoolTaskActive}
            onClose={() => {
              setQuestionPoolTaskActive(false);
              setQuestionPoolCreateOpen(false);
            }}
          />
        </div>
      </Show>
    </PageFrame>
  );
}
