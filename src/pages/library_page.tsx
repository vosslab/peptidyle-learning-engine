// library_page.tsx - injected Question Library browse surface; route wiring follows the server contract.

import { useLocation, useNavigate, useSearchParams } from "@solidjs/router";
import { For, Show, createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import { LibraryBloomDiscovery } from "../components/library_bloom_discovery";
import { QuestionBulkMetadataEditor } from "../components/question_bulk_metadata_editor";
import { QuestionPoolCreateDialog } from "../components/question_pool_create_dialog";
import type { QuestionPoolLibraryClient } from "../api/question_pool_library";
import type { LibraryDiscussionClient } from "../api/library_discussion";
import type { BloomClassificationCorrectionClient } from "../api/bloom_classification";
import { LibraryPoolDiscovery } from "./library_pool_discovery";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import type {
  QuestionBulkMetadataClient,
  QuestionBulkMetadataUpdateResult,
} from "../api/question_bulk_metadata";
import type { QuestionPoolCreationClient } from "../api/question_pool_creation";
import { questionLibraryBulkSelectionRequest } from "../api/question_library_repository";
import { useSessionBootstrap } from "../auth/session_context";
import "./library_page.css";
import { LibraryBrowseControls } from "./library_browse_controls";
import { LibraryBrowseRows } from "./library_browse_rows";
import {
  backendLabel,
  hasCanonicalPoolDeepLink,
  questionTypeLabel,
  RetainedSelectOption,
  selectedQuestionLibrarySort,
} from "./library_page_helpers";
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
  QuestionLibraryBrowseSession,
  clampQuestionLibraryReturnScrollTop,
  createQuestionLibraryReturnToken,
  parseQuestionLibraryReturnToken,
  questionLibraryReturnPath,
  saveQuestionLibraryReturnState,
  takeQuestionLibraryReturnState,
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowseFacetAggregate,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRow,
  type QuestionLibraryBrowseState,
  type QuestionLibraryFacetTruncation,
} from "./library_page_model";

/* Each virtual row reserves room for a Question Title, two-line summary, and Question Authors.
 * Keep this fallback aligned with --ple-question-library-row-block-size in src/style.css. */
const FALLBACK_ROW_HEIGHT_PX = 112;

export interface LibraryPageProps {
  readonly mode: "search" | "browse";
  readonly repository: QuestionLibraryBrowseRepository;
  readonly metadataClient: QuestionBulkMetadataClient;
  readonly classificationClient: import("../api/content_classification").ContentClassificationClient;
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly poolLibraryClient?: QuestionPoolLibraryClient &
    LibraryDiscussionClient &
    BloomClassificationCorrectionClient;
  readonly getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>;
}

/** Question Library UI with the production repository injected by the route composition. */
export function LibraryPage(props: LibraryPageProps): JSX.Element {
  const sessionBootstrapState = useSessionBootstrap().state();
  if (sessionBootstrapState.kind !== "authenticated") {
    throw new Error("Question Library requires an authenticated session scope");
  }
  const sessionScope = sessionBootstrapState.session;
  const mayMutateLibrary = sessionScope.account.productRole === "instructor";
  const location = useLocation();
  const navigate = useNavigate();
  const hasInitialPoolDeepLink = hasCanonicalPoolDeepLink(location.search);
  const [searchParams] = useSearchParams();
  const returnToken = parseQuestionLibraryReturnToken(
    searchParams[QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER],
  );
  const takenReturnState = takeQuestionLibraryReturnState(sessionScope, returnToken);
  const returnState = takenReturnState?.origin === props.mode ? takenReturnState : null;
  // Catch only URL parsing at the route boundary; transport validation remains strict.
  function routeHandoff(search: string): QuestionLibraryBrowseQuery | null {
    try {
      return searchHandoffQuery(search);
    } catch {
      return null;
    }
  }
  const initialHandoffQuery = routeHandoff(location.search);
  const [invalidLinkOptions, setInvalidLinkOptions] = createSignal(
    returnState === null && initialHandoffQuery === null,
  );
  const [query, setQuery] = createSignal<QuestionLibraryBrowseQuery>(
    returnState?.query ?? initialHandoffQuery ?? EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  );
  const [state, setState] = createSignal<QuestionLibraryBrowseState>(
    returnState !== null && !returnState.refreshOnReturn
      ? returnState.browseState
      : {
          kind: "initial",
          rows: [],
          aggregates: [],
          nextCursor: null,
          facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
        },
  );
  const [scrollTop, setScrollTop] = createSignal(returnState?.scrollTop ?? 0);
  const [viewportHeight, setViewportHeight] = createSignal(560);
  const [rowHeightPx, setRowHeightPx] = createSignal(FALLBACK_ROW_HEIGHT_PX);
  const [libraryWindow, setLibraryWindow] = createSignal<HTMLDivElement>();
  const [selectedIds, setSelectedIds] = createSignal<ReadonlySet<string>>(new Set());
  const [selectionNotice, setSelectionNotice] = createSignal<string | null>(null);
  const [editorMetadata, setEditorMetadata] =
    createSignal<ReadonlyArray<PublishedQuestionSharedMetadata> | null>(null);
  const [editorLoading, setEditorLoading] = createSignal(false);
  const [editorLoadError, setEditorLoadError] = createSignal(false);
  const [editorBusy, setEditorBusy] = createSignal(false);
  const [updateResults, setUpdateResults] =
    createSignal<ReadonlyArray<QuestionBulkMetadataUpdateResult> | null>(null);
  const [questionPoolCreateOpen, setQuestionPoolCreateOpen] = createSignal(false);
  const [questionPoolTaskActive, setQuestionPoolTaskActive] = createSignal(false);
  const [poolDiscoveryOpened, setPoolDiscoveryOpened] = createSignal(hasInitialPoolDeepLink);
  let pendingScrollRestore = returnState?.scrollTop ?? null;
  const questionReturnTokens = new Map<string, string>();
  const session = new QuestionLibraryBrowseSession(props.repository, setState);

  createEffect(() => {
    const routeSearch = location.search;
    if (returnState !== null) return;
    const handoffQuery = routeHandoff(routeSearch);
    setInvalidLinkOptions(handoffQuery === null);
    if (handoffQuery === null || props.mode !== "search") return;
    if (!hasExactBrowseFilters(handoffQuery)) return;
    setQuery(handoffQuery);
    setScrollTop(0);
    void session.reset(handoffQuery);
  });

  const aggregates = (): ReadonlyArray<QuestionLibraryBrowseFacetAggregate> => {
    const current = state();
    return current.aggregates;
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
      | "usedInMyCourses"
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
      return current.aggregates.filter((aggregate) => aggregate.facet === facet);
    };
  };
  const displayedRows = (): ReadonlyArray<QuestionLibraryBrowseRow> => {
    const current = state();
    if (props.mode === "browse" && !hasExactBrowseFilters(query())) return [];
    return current.kind === "empty" ? [] : current.rows;
  };
  const facetTruncation = (): QuestionLibraryFacetTruncation => {
    const current = state();
    if ((current.kind === "loading" || current.kind === "error") && current.rows.length === 0) {
      return NO_QUESTION_LIBRARY_FACET_TRUNCATION;
    }
    return current.facetTruncation;
  };
  function changeQuery(change: Partial<QuestionLibraryBrowseQuery>): void {
    if (invalidLinkOptions()) return;
    if (selectedIds().size > 0) {
      setSelectionNotice("Selection cleared because the search, filters, or order changed.");
    }
    setSelectedIds(new Set<string>());
    setEditorMetadata(null);
    setEditorLoadError(false);
    setUpdateResults(null);
    const next = { ...query(), ...change };
    setQuery(next);
    setScrollTop(0);
    void session.reset(next);
  }

  function resetInvalidLinkOptions(): void {
    const search = recoverLibrarySearch(location.search);
    const recoveredQuery = searchHandoffQuery(search);
    setInvalidLinkOptions(false);
    navigate(`${location.pathname}${search}${location.hash}`, { replace: true });
    // Exact search filters are applied by the route effect; other modes need an explicit reset.
    if (props.mode !== "search" || !hasExactBrowseFilters(recoveredQuery)) {
      changeQuery(recoveredQuery);
    }
  }

  function updateSelection(questionId: string, checked: boolean): void {
    const next = new Set(selectedIds());
    if (checked) {
      if (next.size >= MAX_BULK_QUESTION_METADATA_ITEMS) {
        setSelectionNotice(
          `You can select at most ${MAX_BULK_QUESTION_METADATA_ITEMS} Questions at once.`,
        );
        return;
      }
      next.add(questionId);
    } else {
      next.delete(questionId);
    }
    setSelectedIds(next);
    setEditorMetadata(null);
    setEditorLoadError(false);
    setUpdateResults(null);
    setSelectionNotice(null);
  }

  function selectLoadedQuestions(): void {
    const rows = displayedRows();
    const selected = rows.slice(0, MAX_BULK_QUESTION_METADATA_ITEMS).map((row) => row.displayId);
    setSelectedIds(new Set(selected));
    setEditorMetadata(null);
    setEditorLoadError(false);
    setUpdateResults(null);
    setSelectionNotice(
      rows.length > MAX_BULK_QUESTION_METADATA_ITEMS
        ? `Selected the first ${MAX_BULK_QUESTION_METADATA_ITEMS} loaded Questions; the bulk limit is ${MAX_BULK_QUESTION_METADATA_ITEMS}.`
        : `Selected all ${rows.length} Questions currently loaded in this browser.`,
    );
  }

  function clearSelection(): void {
    setSelectedIds(new Set<string>());
    setEditorMetadata(null);
    setEditorLoadError(false);
    setUpdateResults(null);
    setSelectionNotice("Selection cleared.");
  }

  async function openMetadataEditor(): Promise<void> {
    setEditorLoading(true);
    setEditorBusy(true);
    setEditorLoadError(false);
    setUpdateResults(null);
    try {
      const selection = questionLibraryBulkSelectionRequest([...selectedIds()]);
      const current = await props.metadataClient.getCurrentQuestionBulkMetadata(
        selection.questionIds,
      );
      setEditorMetadata(current);
    } catch {
      setEditorLoadError(true);
    } finally {
      setEditorLoading(false);
      setEditorBusy(false);
    }
  }

  function metadataUpdateSucceeded(results: ReadonlyArray<QuestionBulkMetadataUpdateResult>): void {
    setUpdateResults(results);
    setEditorMetadata(null);
    setSelectedIds(new Set<string>());
    setSelectionNotice(null);
    void session.reset(query());
  }

  function returnTokenFor(row: QuestionLibraryBrowseRow): string {
    const existing = questionReturnTokens.get(row.displayId);
    if (existing !== undefined) return existing;
    const token = createQuestionLibraryReturnToken();
    questionReturnTokens.set(row.displayId, token);
    return token;
  }

  function saveReturnState(token: string): void {
    const current = session.state;
    saveQuestionLibraryReturnState(sessionScope, props.mode, token, query(), current, scrollTop());
    // The source history entry receives the same route token, so browser Back
    // and the visible detail-page return link select the same saved view.
    history.replaceState(history.state, "", questionLibraryReturnPath(token));
  }

  createEffect(() => {
    const windowElement = libraryWindow();
    const current = state();
    if (windowElement === undefined || pendingScrollRestore === null || current.kind !== "ready") {
      return;
    }
    const restored = clampQuestionLibraryReturnScrollTop(
      pendingScrollRestore,
      windowElement.scrollHeight,
      windowElement.clientHeight,
    );
    windowElement.scrollTop = restored;
    setScrollTop(restored);
    pendingScrollRestore = null;
  });

  onMount(() => {
    function refreshRowHeight(): void {
      const configured = Number.parseFloat(
        getComputedStyle(document.documentElement).getPropertyValue(
          "--ple-question-library-row-block-size",
        ),
      );
      if (Number.isFinite(configured) && configured > 0) setRowHeightPx(configured);
    }

    refreshRowHeight();
    const observer = new ResizeObserver(refreshRowHeight);
    observer.observe(document.documentElement);
    onCleanup(() => observer.disconnect());
    if (returnState !== null && !returnState.refreshOnReturn) {
      session.restore(returnState.query, returnState.browseState);
    } else if (returnState !== null) {
      void session.reset(returnState.query);
    } else if (props.mode === "browse" && !invalidLinkOptions()) {
      void session.reset(query());
    }
  });

  return (
    <section
      class="page library-page"
      classList={{ "question-pool-task-active": questionPoolTaskActive() }}
      data-route-surface={props.mode === "browse" ? "library-browse" : "library"}
    >
      <p class="eyebrow">Shared educational content</p>
      <h1>{props.mode === "browse" ? "Browse Question Library" : "Search Question Library"}</h1>
      <p class="page-lede">
        {!mayMutateLibrary
          ? "Review published Library content and its recorded improvement activity."
          : props.mode === "browse"
            ? "Explore what the library contains, then narrow from a broad subject to exact topics."
            : "Find a current published question to study, reuse, or assign."}
      </p>
      <Show when={invalidLinkOptions()}>
        <div role="alert">
          <p>
            This Library link has invalid filter or order options. No search has been run. Reset the
            invalid options to continue with the valid options in this link.
          </p>
          <button type="button" onClick={resetInvalidLinkOptions}>
            Reset invalid Library options
          </button>
        </div>
      </Show>
      <Show when={!invalidLinkOptions()}>
        <Show when={props.poolLibraryClient}>
          {(client) => (
            <details
              class="question-library-pool-discovery"
              open={hasInitialPoolDeepLink}
              onToggle={(event) => {
                if (event.currentTarget.open) setPoolDiscoveryOpened(true);
              }}
            >
              <summary>Discover Published Question Pools</summary>
              <Show when={poolDiscoveryOpened()}>
                <LibraryPoolDiscovery
                  client={client()}
                  classificationClient={props.classificationClient}
                  mayWatchPools={mayMutateLibrary}
                  mayCorrectBloom={mayMutateLibrary}
                />
              </Show>
            </details>
          )}
        </Show>
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
          <form
            class="question-library-controls"
            classList={{ "question-library-controls-initial": state().kind === "initial" }}
            onSubmit={(event) => event.preventDefault()}
          >
            <label class="question-library-search-control">
              Search published questions
              <input
                type="search"
                value={query().search}
                onInput={(event) => changeQuery({ search: event.currentTarget.value })}
                placeholder="Title or concept"
                disabled={editorBusy()}
              />
            </label>
            <Show when={state().kind !== "initial"}>
              <LibraryClassificationSearch
                value={query()}
                client={props.classificationClient}
                disabled={editorBusy()}
                onChange={changeQuery}
              />
            </Show>
            <details class="question-library-search-tips">
              <summary>Search tips</summary>
              <div>
                <p>
                  Ordinary words search together. Use quotes for a phrase and a leading minus to
                  exclude.
                </p>
                <p>
                  Fields: <code>discipline:</code>, <code>subject:</code>, <code>topic:</code>,{" "}
                  <code>subtopic:</code>, <code>tags:</code>, <code>type:</code>, and{" "}
                  <code>author:</code>.
                </p>
                <ul aria-label="Search examples">
                  <li>
                    <code>topic:genetics</code>
                  </li>
                  <li>
                    <code>tags:&quot;cell division&quot;</code>
                  </li>
                  <li>
                    <code>type:&quot;multiple choice&quot;</code>
                  </li>
                  <li>
                    <code>author:&quot;Ada Lovelace&quot;</code>
                  </li>
                  <li>
                    <code>meiosis -mitosis</code>
                  </li>
                  <li>
                    <code>&quot;cell membrane&quot;</code>
                  </li>
                </ul>
              </div>
            </details>
            <Show when={state().kind !== "initial"}>
              <label>
                Question Author
                <select
                  value={query().authorName ?? ""}
                  onChange={(event) =>
                    changeQuery({ authorName: event.currentTarget.value || null })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">All Question Authors</option>
                  <RetainedSelectOption value={query().authorName} label={(value) => value} />
                  <For
                    each={facets("authorName")().filter(
                      (facet) => facet.value !== query().authorName,
                    )}
                  >
                    {(facet) => (
                      <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
                <Show when={facetTruncation().authorNames}>
                  <span class="question-library-filter-truncated">
                    More Question Authors match. Narrow the search or use <code>author:</code>.
                  </span>
                </Show>
              </label>
              <label>
                Backend
                <select
                  value={query().backend ?? ""}
                  onChange={(event) => changeQuery({ backend: event.currentTarget.value || null })}
                  disabled={editorBusy()}
                >
                  <option value="">All backends</option>
                  <RetainedSelectOption value={query().backend} label={backendLabel} />
                  <For
                    each={facets("backend")().filter((facet) => facet.value !== query().backend)}
                  >
                    {(facet) => (
                      <option
                        value={facet.value}
                      >{`${backendLabel(facet.value)} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
              </label>
              <label>
                Tag
                <select
                  value={query().tag ?? ""}
                  onChange={(event) => changeQuery({ tag: event.currentTarget.value || null })}
                  disabled={editorBusy()}
                >
                  <option value="">All tags</option>
                  <RetainedSelectOption value={query().tag} label={(value) => value} />
                  <For each={facets("tag")().filter((facet) => facet.value !== query().tag)}>
                    {(facet) => (
                      <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
                <Show when={facetTruncation().tags}>
                  <span class="question-library-filter-truncated">
                    More tags match. Narrow the search or use <code>tags:</code>.
                  </span>
                </Show>
              </label>
              <label>
                Subject name (additional filter)
                <select
                  value={query().subjects[0] ?? ""}
                  onChange={(event) =>
                    changeQuery({
                      subjects: event.currentTarget.value ? [event.currentTarget.value] : [],
                      topics: [],
                    })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">All subjects</option>
                  <RetainedSelectOption value={query().subjects[0]} label={(value) => value} />
                  <For
                    each={facets("subject")().filter(
                      (facet) => facet.value !== query().subjects[0],
                    )}
                  >
                    {(facet) => (
                      <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
                <Show when={facetTruncation().subjects}>
                  <span class="question-library-filter-truncated">
                    More subjects match. Narrow the search or use <code>subject:</code>.
                  </span>
                </Show>
              </label>
              <label>
                Topic name (additional filter)
                <select
                  value={query().topics[0] ?? ""}
                  onChange={(event) =>
                    changeQuery({
                      topics: event.currentTarget.value ? [event.currentTarget.value] : [],
                    })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">All topics</option>
                  <RetainedSelectOption value={query().topics[0]} label={(value) => value} />
                  <For
                    each={facets("topic")().filter((facet) => facet.value !== query().topics[0])}
                  >
                    {(facet) => (
                      <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
                <Show when={facetTruncation().topics}>
                  <span class="question-library-filter-truncated">
                    More topics match. Narrow the search or use <code>topic:</code>.
                  </span>
                </Show>
              </label>
              <label>
                Question Type
                <select
                  value={query().questionType ?? ""}
                  onChange={(event) =>
                    changeQuery({ questionType: event.currentTarget.value || null })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">All Question Types</option>
                  <RetainedSelectOption value={query().questionType} label={questionTypeLabel} />
                  <For
                    each={facets("questionType")().filter(
                      (facet) => facet.value !== query().questionType,
                    )}
                  >
                    {(facet) => (
                      <option
                        value={facet.value}
                      >{`${questionTypeLabel(facet.value)} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
              </label>
              <label>
                Question License
                <select
                  value={query().questionLicense ?? ""}
                  onChange={(event) =>
                    changeQuery({ questionLicense: event.currentTarget.value || null })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">All Question Licenses</option>
                  <RetainedSelectOption value={query().questionLicense} label={(value) => value} />
                  <For
                    each={facets("questionLicense")().filter(
                      (facet) => facet.value !== query().questionLicense,
                    )}
                  >
                    {(facet) => (
                      <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
              </label>
              <label>
                Used in my courses
                <select
                  value={query().usedInMyCourses ?? ""}
                  onChange={(event) =>
                    changeQuery({ usedInMyCourses: event.currentTarget.value || null })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">Any course use</option>
                  <RetainedSelectOption
                    value={query().usedInMyCourses}
                    label={() => "Used in my courses"}
                  />
                  <For
                    each={facets("usedInMyCourses")().filter(
                      (facet) => facet.value !== query().usedInMyCourses,
                    )}
                  >
                    {(facet) => (
                      <option value={facet.value}>{`Used in my courses (${facet.count})`}</option>
                    )}
                  </For>
                </select>
              </label>
              <label>
                Capability
                <select
                  value={query().capability ?? ""}
                  onChange={(event) =>
                    changeQuery({ capability: event.currentTarget.value || null })
                  }
                  disabled={editorBusy()}
                >
                  <option value="">All capabilities</option>
                  <RetainedSelectOption value={query().capability} label={(value) => value} />
                  <For
                    each={facets("capability")().filter(
                      (facet) => facet.value !== query().capability,
                    )}
                  >
                    {(facet) => (
                      <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                    )}
                  </For>
                </select>
              </label>
            </Show>
          </form>
        </Show>
        <Show when={props.mode === "browse"}>
          <div class="question-library-controls" role="group" aria-label="Classification filters">
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
            startOver={() => changeQuery(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY)}
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
          <div class="question-library-controls" role="group" aria-label="Result order">
            <label>
              Order results
              <select
                value={query().sort}
                onChange={(event) =>
                  changeQuery({ sort: selectedQuestionLibrarySort(event.currentTarget.value) })
                }
                disabled={editorBusy()}
              >
                <option value="titleAscending">Title (A-Z)</option>
                <option value="publishedNewest">Recently published</option>
              </select>
            </label>
          </div>
        </Show>
        <LibraryBrowseRows
          mayMutateLibrary={mayMutateLibrary}
          displayedRows={displayedRows}
          selectedIds={selectedIds}
          editorBusy={editorBusy}
          browseState={state}
          scrollTop={scrollTop}
          viewportHeight={viewportHeight}
          rowHeightPx={rowHeightPx}
          setLibraryWindow={setLibraryWindow}
          onScroll={(nextScrollTop, nextViewportHeight) => {
            setScrollTop(nextScrollTop);
            setViewportHeight(nextViewportHeight);
          }}
          onNeedMore={() => void session.loadNext()}
          onRetry={() => void session.retry()}
          onUpdateSelection={updateSelection}
          onSelectLoaded={selectLoadedQuestions}
          onClearSelection={clearSelection}
          onOpenMetadataEditor={() => void openMetadataEditor()}
          returnTokenFor={returnTokenFor}
          onSaveReturnState={saveReturnState}
        >
          <Show when={mayMutateLibrary && questionPoolCreateOpen()}>
            <div class="question-pool-create-host">
              <QuestionPoolCreateDialog
                questionPoolClient={props.questionPoolClient}
                questionLibrary={props.repository}
                getQuestionDetails={props.getQuestionDetails}
                onTaskPhaseChange={setQuestionPoolTaskActive}
                onClose={() => {
                  setQuestionPoolTaskActive(false);
                  setQuestionPoolCreateOpen(false);
                }}
              />
            </div>
          </Show>
          <Show when={selectionNotice()}>{(notice) => <p role="status">{notice()}</p>}</Show>
          <Show when={editorLoading()}>
            <p class="loading-state" role="status">
              Reading current metadata for all selected Questions...
            </p>
          </Show>
          <Show when={mayMutateLibrary && editorLoadError()}>
            <section class="route-error" role="alert">
              <h2>Current metadata could not be loaded</h2>
              <p>Your selection is preserved. Retry the read before editing.</p>
              <button
                class="primary-action"
                type="button"
                onClick={() => void openMetadataEditor()}
              >
                Retry current metadata
              </button>
            </section>
          </Show>
          <Show when={mayMutateLibrary && editorMetadata()}>
            {(metadata) => (
              <QuestionBulkMetadataEditor
                client={props.metadataClient}
                classificationClient={props.classificationClient}
                initialMetadata={metadata()}
                onBusyChange={setEditorBusy}
                onCancel={() => setEditorMetadata(null)}
                onSuccess={metadataUpdateSucceeded}
              />
            )}
          </Show>
          <Show when={updateResults()}>
            {(results) => (
              <section class="question-library-bulk-success" role="status">
                <h2>Updated shared metadata for {results().length} Questions</h2>
                <p>The selection was cleared and the current library search is refreshing.</p>
              </section>
            )}
          </Show>
        </LibraryBrowseRows>
      </Show>
    </section>
  );
}
