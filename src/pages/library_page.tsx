// library_page.tsx - injected Question Library browse surface; route wiring follows the server contract.

import { A, useLocation, useNavigate, useSearchParams } from "@solidjs/router";
import { For, Show, createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import { CopyableQuestionId } from "../components/copyable_question_id";
import { QuestionBulkMetadataEditor } from "../components/question_bulk_metadata_editor";
import { QuestionPoolCreateDialog } from "../components/question_pool_create_dialog";
import type { QuestionPoolLibraryClient } from "../api/question_pool_library";
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
import { buildRoutePath } from "../ribbon/ribbon_contract";
import "./library_page.css";
import { LibraryBrowseControls } from "./library_browse_controls";
import { LibraryClassificationSearch } from "../components/library_classification_search";
import {
  searchHandoffQuery,
  hasExactBrowseFilters,
  searchWithinResultsPath,
  clearLibraryClassificationSearch,
} from "./library_search_parameters";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  QuestionLibraryBrowseSession,
  clampQuestionLibraryReturnScrollTop,
  createQuestionLibraryReturnToken,
  parseQuestionLibraryReturnToken,
  questionLibraryBrowseVirtualWindow,
  questionLibraryReturnPath,
  saveQuestionLibraryReturnState,
  takeQuestionLibraryReturnState,
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRow,
  type QuestionLibraryBrowseState,
  type QuestionLibraryFacetTruncation,
} from "./library_page_model";

/* Each virtual row reserves room for a Question Title, two-line summary, and Question Authors.
 * Keep this fallback aligned with --ple-question-library-row-block-size in src/style.css. */
const FALLBACK_ROW_HEIGHT_PX = 112;
const OVERSCAN_ROWS = 5;
const DRAFT_QUESTIONS_PATH = buildRoutePath("questionDrafts", {});
function questionLink(row: QuestionLibraryBrowseRow, returnToken: string): string {
  return `/library/${encodeURIComponent(row.displayId)}?${new URLSearchParams({
    [QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER]: returnToken,
  }).toString()}`;
}

function questionTypeLabel(value: string): string {
  const labels: Readonly<Record<string, string>> = {
    multipleChoice: "Multiple choice",
    multipleAnswer: "Multiple answer",
    fillInBlank: "Fill in the blank",
    multipleFillInBlank: "Multiple fill in the blank",
    numeric: "Numeric",
    matching: "Matching",
    ordering: "Ordering",
    hotspot: "Hotspot",
  };
  return labels[value] ?? value;
}

function backendLabel(value: string): string {
  const labels: Readonly<Record<string, string>> = {
    ple: "PLE",
    webwork: "WeBWorK",
    imathas: "IMathAS",
  };
  return labels[value] ?? value;
}

function RetainedSelectOption(props: {
  readonly value: string | null | undefined;
  readonly label: (value: string) => string;
}): JSX.Element {
  return (
    <Show when={props.value}>
      {(value) => (
        <option value={value()} selected>
          {props.label(value())}
        </option>
      )}
    </Show>
  );
}

function webworkFormatLabel(value: QuestionLibraryBrowseRow["questionFormat"]): string | null {
  if (value === "webworkPg") return "PG";
  if (value === "webworkPgml") return "PGML";
  return null;
}

export interface LibraryPageProps {
  readonly mode: "search" | "browse";
  readonly repository: QuestionLibraryBrowseRepository;
  readonly metadataClient: QuestionBulkMetadataClient;
  readonly classificationClient: import("../api/content_classification").ContentClassificationClient;
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly poolLibraryClient?: QuestionPoolLibraryClient;
  readonly getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>;
}

/** Question Library UI with the production repository injected by the route composition. */
export function LibraryPage(props: LibraryPageProps): JSX.Element {
  const sessionBootstrapState = useSessionBootstrap().state();
  if (sessionBootstrapState.kind !== "authenticated") {
    throw new Error("Question Library requires an authenticated session scope");
  }
  const sessionScope = sessionBootstrapState.session;
  const location = useLocation();
  const navigate = useNavigate();
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
  const [invalidClassification, setInvalidClassification] = createSignal(
    returnState === null && initialHandoffQuery === null,
  );
  const [query, setQuery] = createSignal<QuestionLibraryBrowseQuery>(
    returnState?.query ?? initialHandoffQuery ?? EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  );
  const [state, setState] = createSignal<QuestionLibraryBrowseState>(
    returnState?.browseState ?? {
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
  const [poolDiscoveryOpened, setPoolDiscoveryOpened] = createSignal(false);
  let pendingScrollRestore = returnState?.scrollTop ?? null;
  const questionReturnTokens = new Map<string, string>();
  const session = new QuestionLibraryBrowseSession(props.repository, setState);

  createEffect(() => {
    const routeSearch = location.search;
    if (returnState !== null) return;
    const handoffQuery = routeHandoff(routeSearch);
    setInvalidClassification(handoffQuery === null);
    if (handoffQuery === null || props.mode !== "search") return;
    if (!hasExactBrowseFilters(handoffQuery)) return;
    setQuery(handoffQuery);
    setScrollTop(0);
    void session.reset(handoffQuery);
  });

  const ready = (): Extract<QuestionLibraryBrowseState, { readonly kind: "ready" }> | undefined => {
    const current = state();
    return current.kind === "ready" ? current : undefined;
  };
  const aggregates = (): ReadonlyArray<{
    readonly facet: string;
    readonly value: string;
    readonly count: number;
  }> => {
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
      | "usedInMyCourses",
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
  const browsingGroupsLoading = (): boolean => {
    const current = state();
    return current.kind === "loading" && current.rows.length === 0;
  };
  const virtualWindow = (): Readonly<{
    readonly offset: number;
    readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
  }> =>
    questionLibraryBrowseVirtualWindow(
      displayedRows(),
      scrollTop(),
      viewportHeight(),
      rowHeightPx(),
      OVERSCAN_ROWS,
    );

  function changeQuery(change: Partial<QuestionLibraryBrowseQuery>): void {
    if (invalidClassification()) return;
    if (selectedIds().size > 0) {
      setSelectionNotice("Selection cleared because the search or filters changed.");
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

  function clearInvalidClassification(): void {
    const search = clearLibraryClassificationSearch(location.search);
    const recoveredQuery = searchHandoffQuery(search);
    setInvalidClassification(false);
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

  function handleScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLDivElement)) {
      return;
    }
    setScrollTop(target.scrollTop);
    setViewportHeight(target.clientHeight);
    const current = ready();
    if (
      current !== undefined &&
      target.scrollTop + target.clientHeight >= target.scrollHeight - rowHeightPx() * 3
    ) {
      void session.loadNext();
    }
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
    if (returnState !== null) {
      session.restore(returnState.query, returnState.browseState);
    } else if (props.mode === "browse" && !invalidClassification()) {
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
        {props.mode === "browse"
          ? "Explore what the library contains, then narrow from a broad subject to exact topics."
          : "Find a current published question to study, reuse, or assign."}
      </p>
      <Show when={invalidClassification()}>
        <div role="alert">
          <p>
            This Library link has invalid classification filters. No search has been run. Clear the
            classification filters to continue with the other filters in this link.
          </p>
          <button type="button" onClick={clearInvalidClassification}>
            Clear classification filters
          </button>
        </div>
      </Show>
      <Show when={!invalidClassification()}>
        <Show when={props.poolLibraryClient}>
          {(client) => (
            <details
              class="question-library-pool-discovery"
              onToggle={(event) => {
                if (event.currentTarget.open) setPoolDiscoveryOpened(true);
              }}
            >
              <summary>Discover Published Question Pools</summary>
              <Show when={poolDiscoveryOpened()}>
                <LibraryPoolDiscovery
                  client={client()}
                  classificationClient={props.classificationClient}
                />
              </Show>
            </details>
          )}
        </Show>
        <Show when={sessionScope.account.productRole === "instructor"}>
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
            browsingGroupsLoading={browsingGroupsLoading}
            browseFacets={browseFacets}
            facetTruncation={facetTruncation}
            questionTypeLabel={questionTypeLabel}
            changeQuery={changeQuery}
            startOver={() => changeQuery(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY)}
          />
        </Show>
        <Show when={displayedRows().length > 0 || selectedIds().size > 0}>
          <section class="question-library-bulk-toolbar" aria-label="Bulk Question actions">
            <p aria-live="polite">
              <strong>{selectedIds().size} selected</strong> from {displayedRows().length} loaded
              Questions
            </p>
            <div>
              <button
                type="button"
                class="quiet-action"
                disabled={editorBusy() || displayedRows().length === 0}
                onClick={selectLoadedQuestions}
              >
                Select loaded Questions
              </button>
              <button
                type="button"
                class="quiet-action"
                disabled={editorBusy() || selectedIds().size === 0}
                onClick={clearSelection}
              >
                Clear selection
              </button>
              <button
                type="button"
                class="primary-action"
                disabled={editorBusy() || selectedIds().size === 0}
                onClick={() => void openMetadataEditor()}
              >
                Edit shared metadata
              </button>
            </div>
            <p class="question-library-bulk-help">
              Select loaded Questions affects only results fetched into this browser, never every
              Question in the library. Each bulk update is limited to{" "}
              {MAX_BULK_QUESTION_METADATA_ITEMS}.
            </p>
          </section>
        </Show>
        <Show when={questionPoolCreateOpen()}>
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
        <Show when={editorLoadError()}>
          <section class="route-error" role="alert">
            <h2>Current metadata could not be loaded</h2>
            <p>Your selection is preserved. Retry the read before editing.</p>
            <button class="primary-action" type="button" onClick={() => void openMetadataEditor()}>
              Retry current metadata
            </button>
          </section>
        </Show>
        <Show when={editorMetadata()}>
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
        <Show when={state().kind === "error"}>
          <section class="route-error" role="alert">
            <h2>The library could not load</h2>
            <p>Your filters are still here. Check the connection and try again.</p>
            <button class="primary-action" type="button" onClick={() => void session.retry()}>
              Try again
            </button>
          </section>
        </Show>
        <Show when={state().kind === "empty"}>
          <section class="empty-state" aria-label="No matching published questions">
            <h2>No published questions match these filters</h2>
            <p>Use the global Question Library to find and reuse published Questions.</p>
            <p>Try a shorter search or choose a broader topic.</p>
            <Show when={DRAFT_QUESTIONS_PATH}>
              {(path) => (
                <A class="primary-action" href={path()} children="Create a Draft Question" />
              )}
            </Show>
          </section>
        </Show>
        <Show when={displayedRows().length > 0}>
          <div
            class="question-library-window"
            role="region"
            aria-label="Published questions"
            tabIndex={0}
            ref={setLibraryWindow}
            onScroll={handleScroll}
            style={`--ple-question-library-loaded-block-size:${displayedRows().length * rowHeightPx()}px`}
          >
            <div
              style={{
                height: `${displayedRows().length * rowHeightPx()}px`,
                position: "relative",
              }}
            >
              <div
                class="question-library-window-slice"
                style={{ top: `${virtualWindow().offset}px` }}
              >
                <For each={virtualWindow().rows}>
                  {(row) => (
                    <article class="question-library-row" style={{ height: `${rowHeightPx()}px` }}>
                      <label class="question-library-row-selection">
                        <input
                          type="checkbox"
                          checked={selectedIds().has(row.displayId)}
                          disabled={
                            editorBusy() ||
                            (!selectedIds().has(row.displayId) &&
                              selectedIds().size >= MAX_BULK_QUESTION_METADATA_ITEMS)
                          }
                          onChange={(event) =>
                            updateSelection(row.displayId, event.currentTarget.checked)
                          }
                        />
                        <span class="sr-only">Select {row.questionTitle}</span>
                      </label>
                      <h2>{row.questionTitle}</h2>
                      <p class="question-library-row-summary">{row.summary}</p>
                      <p class="question-library-row-authors" aria-label="Question Authors">
                        Authors: {row.authorNames.join(", ")}
                        <Show when={webworkFormatLabel(row.questionFormat)}>
                          {(format) => <> · Format: {format()}</>}
                        </Show>
                      </p>
                      <CopyableQuestionId
                        questionTitle={row.questionTitle}
                        displayId={row.displayId}
                        presentation="compact"
                      />
                      <A
                        class="quiet-link"
                        href={questionLink(row, returnTokenFor(row))}
                        onClick={(event) => {
                          if (
                            event.button !== 0 ||
                            event.metaKey ||
                            event.ctrlKey ||
                            event.shiftKey ||
                            event.altKey
                          ) {
                            return;
                          }
                          const token = new URL(event.currentTarget.href).searchParams.get(
                            QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
                          );
                          if (token !== null) saveReturnState(token);
                        }}
                      >
                        Open question
                      </A>
                    </article>
                  )}
                </For>
              </div>
            </div>
            <Show when={state().kind === "loading"}>
              <p class="loading-state" role="status">
                Loading more published questions...
              </p>
            </Show>
          </div>
        </Show>
        <Show when={state().kind === "loading" && displayedRows().length === 0}>
          <p class="loading-state" role="status">
            Loading published questions...
          </p>
        </Show>
      </Show>
    </section>
  );
}
