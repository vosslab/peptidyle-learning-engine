// library_page.tsx - injected Question Library browse surface; route wiring follows the server contract.

import { useLocation, useNavigate } from "@solidjs/router";
import { Show, createEffect, createSignal, type JSX } from "solid-js";

import { LibraryBloomDiscovery } from "../components/library_bloom_discovery";
import { createLibraryBulkActions, LibraryBulkActions } from "./library_bulk_actions";
import { QuestionPoolCreateDialog } from "../components/question_pool_create_dialog";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { QuestionPoolSearchMetadataClient } from "../api/question_pool_search_metadata";
import type { QuestionPoolCreationClient } from "../api/question_pool_creation";
import { useSessionBootstrap } from "../auth/session_context";
import "./library_page.css";
import { createSearchState } from "../features/search/search_state";
import { SearchControls } from "../features/search/search_controls";
import { SearchResults } from "../features/search/search_results";
import { SearchPage } from "../features/search/search_page";
import { questionLibrarySearchDefinition } from "./question_library_search_definition";
import { LibraryBrowseControls } from "./library_browse_controls";
import { LibrarySearchFilters } from "./library_search_filters";
import { questionTypeLabel } from "./library_page_helpers";
import { LibraryClassificationSearch } from "../components/library_classification_search";
import {
  searchHandoffQuery,
  hasExactBrowseFilters,
  searchWithinResultsPath,
  recoverLibrarySearch,
} from "./library_search_parameters";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowseFacetAggregate,
  type QuestionLibraryBrowseQuery,
  type LibrarySearchRow,
  type QuestionLibraryFacetTruncation,
} from "./library_page_model";

export interface LibraryPageProps {
  readonly mode: "search" | "browse";
  readonly repository: QuestionLibraryBrowseRepository;
  readonly metadataClient: QuestionBulkMetadataClient;
  readonly poolMetadataClient: QuestionPoolSearchMetadataClient;
  readonly classificationClient: import("../api/content_classification").ContentClassificationClient;
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>;
}

/** Question Library UI with the production repository injected by the route composition. */
export function LibraryPage(props: LibraryPageProps): JSX.Element {
  const sessionBootstrapState = useSessionBootstrap().state();
  if (sessionBootstrapState.kind !== "authenticated") {
    throw new Error("Question Library requires an authenticated session scope");
  }
  const sessionScope = sessionBootstrapState.session;
  const mayMutateLibrary = sessionScope.account.userRole === "instructor";
  const location = useLocation();
  const navigate = useNavigate();
  // Catch only URL parsing at the route boundary; transport validation remains strict.
  function routeHandoff(search: string): QuestionLibraryBrowseQuery | null {
    try {
      return searchHandoffQuery(search);
    } catch {
      return null;
    }
  }
  const initialHandoffQuery = routeHandoff(location.search);
  const [invalidLinkOptions, setInvalidLinkOptions] = createSignal(initialHandoffQuery === null);
  const searchState = createSearchState(questionLibrarySearchDefinition(props.repository), {
    initialQuery: initialHandoffQuery ?? EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  });
  const query = searchState.query;
  const state = searchState.state;
  const selectedIds = searchState.selectedIds;
  const [questionPoolCreateOpen, setQuestionPoolCreateOpen] = createSignal(false);
  const [questionPoolTaskActive, setQuestionPoolTaskActive] = createSignal(false);
  createEffect(() => {
    const routeSearch = location.search;
    const handoffQuery = routeHandoff(routeSearch);
    setInvalidLinkOptions(handoffQuery === null);
    if (handoffQuery === null) return;
    if (props.mode === "browse" || routeSearch.length > 0) void searchState.open(handoffQuery);
  });

  const aggregates = (): ReadonlyArray<QuestionLibraryBrowseFacetAggregate> => {
    const current = state();
    return current.filterCounts?.aggregates ?? [];
  };
  const facets = (
    facet:
      | "authorName"
      | "backend"
      | "tag"
      | "subject"
      | "topic"
      | "questionType"
      | "capability"
      | "questionLicense"
      | "bloomCognitiveProcess"
      | "bloomKnowledgeDimension",
  ): (() => ReadonlyArray<{ readonly value: string; readonly count: number }>) => {
    return () => aggregates().filter((aggregate) => aggregate.facet === facet);
  };
  const browseFacets = (
    facet: "subject" | "topic" | "tag" | "questionType",
  ): (() => ReadonlyArray<{ readonly value: string; readonly count: number }>) => {
    return () => {
      const current = state();
      if ((current.kind === "loading" || current.kind === "error") && current.rows.length === 0) {
        return [];
      }
      return (current.filterCounts?.aggregates ?? []).filter(
        (aggregate) => aggregate.facet === facet,
      );
    };
  };
  const displayedRows = (): ReadonlyArray<LibrarySearchRow> => {
    const current = state();
    if (props.mode === "browse" && !hasExactBrowseFilters(query())) return [];
    return current.kind === "empty" ? [] : current.rows;
  };
  const bulkActions = createLibraryBulkActions({
    state: searchState,
    displayedRows,
    metadataClient: props.metadataClient,
    refresh: searchState.refresh,
  });
  const editorBusy = bulkActions.busy;
  let lastAppliedQuery = query();
  createEffect(() => {
    const current = query();
    if (current === lastAppliedQuery) return;
    lastAppliedQuery = current;
    bulkActions.clearForQueryChange();
  });
  const facetTruncation = (): QuestionLibraryFacetTruncation => {
    const current = state();
    if ((current.kind === "loading" || current.kind === "error") && current.rows.length === 0) {
      return NO_QUESTION_LIBRARY_FACET_TRUNCATION;
    }
    return current.filterCounts?.facetTruncation ?? NO_QUESTION_LIBRARY_FACET_TRUNCATION;
  };
  function changeQuery(change: Partial<QuestionLibraryBrowseQuery>): void {
    if (invalidLinkOptions()) return;
    bulkActions.clearForQueryChange();
    const next = { ...query(), ...change };
    void searchState.apply(next);
  }

  function startOverBrowse(): void {
    bulkActions.clearForQueryChange();
    void searchState.open(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY);
  }

  function resetInvalidLinkOptions(): void {
    const search = recoverLibrarySearch(location.search);
    const recoveredQuery = searchHandoffQuery(search);
    setInvalidLinkOptions(false);
    navigate(`${location.pathname}${search}${location.hash}`, { replace: true });
    if (props.mode === "browse" || search.length > 0) {
      void searchState.open(recoveredQuery);
    } else {
      searchState.clear();
    }
  }

  return (
    <SearchPage
      state={searchState}
      // Library controls, browse groups, and the active pool-creation review.
      contentClass={`library-page${questionPoolTaskActive() ? " question-pool-task-active" : ""}`}
      routeSurface={props.mode === "browse" ? "library-browse" : "library"}
      eyebrow="Shared educational content"
      title={props.mode === "browse" ? "Browse Question Library" : "Search Question Library"}
      lede={
        !mayMutateLibrary
          ? "Review published Library content and its recorded improvement activity."
          : props.mode === "browse"
            ? "Explore what the library contains, then narrow from a broad subject to exact topics."
            : "Find a current published question to study, reuse, or assign."
      }

      toolbar={
        <>
          <Show when={invalidLinkOptions()}>
            <div role="alert">
              <p>
                This Library link has invalid filter or order options. No search has been run. Reset
                the invalid options to continue with the valid options in this link.
              </p>
              <button type="button" onClick={resetInvalidLinkOptions}>
                Reset invalid Library options
              </button>
            </div>
          </Show>
          <Show when={!invalidLinkOptions()}>
            <div class="library-page__filters">
              <Show when={mayMutateLibrary}>
                <p>
                  <button
                    type="button"
                    class="primary-action"
                    disabled={editorBusy()}
                    onClick={() => setQuestionPoolCreateOpen(true)}
                  >
                    Create Question Pool
                  </button>
                </p>
              </Show>
              <p class="sr-only" role="status" aria-live="polite">
                {state().kind === "loading" ? "Loading Question Library results." : ""}
              </p>
              <Show when={props.mode === "search"}>
                <SearchControls
                  state={searchState}
                  textLabel="Search Question Library"
                  textPlaceholder="Title or concept"
                  displayAriaLabel="Library result display"
                  disabled={editorBusy}
                >
                  <Show when={state().kind !== "initial"}>
                    <LibrarySearchFilters
                      query={query}
                      facets={facets}
                      truncation={facetTruncation}
                      classificationClient={props.classificationClient}
                      disabled={editorBusy}
                      onChange={changeQuery}
                    />
                  </Show>
                  <details class="question-library-search-tips">
                    <summary>Search tips</summary>
                    <div>
                      <p>
                        Ordinary words search together. Use quotes for a phrase and a leading minus
                        to exclude.
                      </p>
                      <p>
                        Fields: <code>discipline:</code>, <code>subject:</code>, <code>topic:</code>
                        , <code>subtopic:</code>, <code>tags:</code>, <code>type:</code>, and{" "}
                        <code>author:</code>.
                      </p>
                    </div>
                  </details>
                </SearchControls>
              </Show>
              <Show when={props.mode === "browse"}>
                <div
                  class="question-library-controls"
                  role="group"
                  aria-label="Classification filters"
                >
                  <LibraryClassificationSearch
                    value={query()}
                    client={props.classificationClient}
                    disabled={editorBusy()}
                    onChange={changeQuery}
                  />
                </div>
                <LibraryBrowseControls
                  query={query}
                  hasExactBrowseFilters={() => hasExactBrowseFilters(query())}
                  searchWithinResultsPath={() => searchWithinResultsPath(query())}
                  browsingState={state}
                  browseFacets={browseFacets}
                  facetTruncation={facetTruncation}
                  questionTypeLabel={questionTypeLabel}
                  changeQuery={changeQuery}
                  startOver={startOverBrowse}
                />
              </Show>
              <Show when={props.mode === "browse" || state().kind !== "initial"}>
                <LibraryBloomDiscovery
                  query={query}
                  aggregates={aggregates}
                  reportAvailable={state().kind === "ready" || state().kind === "empty"}
                  disabled={editorBusy()}
                  onChange={changeQuery}
                />
              </Show>
            </div>
          </Show>
        </>
      }
      results={
        <Show
          when={
            !invalidLinkOptions() &&
            (props.mode === "search" || hasExactBrowseFilters(query()) || state().kind === "error")
          }
        >
          <div class="library-page__results">
            <Show when={mayMutateLibrary}>
              <LibraryBulkActions
                controller={bulkActions}
                metadataClient={props.metadataClient}
                poolMetadataClient={props.poolMetadataClient}
                classificationClient={props.classificationClient}
                selectedCount={() => selectedIds().size}
                loadedCount={() => displayedRows().length}
              />
            </Show>
            <SearchResults
              state={searchState}
              ariaLabel="Question Library results"
              disabled={editorBusy}
              rows={displayedRows}
              emptyState={{
                title: "No Library objects match these filters",
                message:
                  "The Question Library contains Published Questions and Question Pools. Try a shorter search or choose a broader topic.",
              }}
              selection={() =>
                mayMutateLibrary
                  ? {
                      kind: "checkbox" as const,
                      selectedIds,
                      disabled: () => editorBusy(),
                      onChange: bulkActions.updateSelection,
                    }
                  : undefined
              }
            />
            <Show when={bulkActions.selectionNotice()}>
              {(notice) => <p role="status">{notice()}</p>}
            </Show>
          </div>
        </Show>
      }
      extra={
        <Show when={mayMutateLibrary && questionPoolCreateOpen()}>
          <div class="question-pool-create-host">
            <QuestionPoolCreateDialog
              questionPoolClient={props.questionPoolClient}
              questionLibrary={props.repository}
              getQuestionDetails={props.getQuestionDetails}
              getCurrentQuestionSharedMetadata={
                props.metadataClient.getCurrentQuestionSharedMetadata
              }
              onTaskPhaseChange={setQuestionPoolTaskActive}
              onClose={() => {
                setQuestionPoolTaskActive(false);
                setQuestionPoolCreateOpen(false);
              }}
            />
          </div>
        </Show>
      }
    />
  );
}
