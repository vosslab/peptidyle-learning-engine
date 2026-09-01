// library_page.tsx - injected Question Library browse surface; route wiring follows the server contract.

import { A } from "@solidjs/router";
import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import { CopyableQuestionId } from "../components/copyable_question_id";
import { QuestionCurationPanel } from "../features/question_curation/question_curation_panel";
import type { QuestionCurationRepository } from "../features/question_curation/question_curation_model";
import type { QuestionPickerSourceRepository } from "../features/question_picker";
import "./library_page.css";
import {
  EMPTY_QUESTION_SEARCH_QUERY,
  QuestionSearchSession,
  questionSearchVirtualWindow,
  type QuestionLibraryRepository,
  type QuestionSearchQuery,
  type QuestionSearchResult,
  type QuestionSearchState,
} from "./library_page_model";

/* Each virtual row reserves room for a title, two-line summary, Question Authors, and classification.
 * Keep this fallback aligned with --ple-question-library-row-block-size in src/style.css. */
const FALLBACK_ROW_HEIGHT_PX = 164;
const OVERSCAN_ROWS = 5;
const percentage = new Intl.NumberFormat("en-US", { style: "percent", maximumFractionDigits: 0 });
const wholeNumber = new Intl.NumberFormat("en-US");
const decimalNumber = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });

function questionLink(row: QuestionSearchResult): string {
  return `/library/${encodeURIComponent(row.displayId)}`;
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
    qti: "QTI",
    h5p: "H5P",
    imathas: "IMathAS",
  };
  return labels[value] ?? value;
}

function QuestionStatisticsPreview(props: { readonly row: QuestionSearchResult }): JSX.Element {
  const availableEvidence = ():
    Extract<QuestionSearchResult["evidence"], { readonly state: "available" }> | undefined =>
    props.row.evidence.state === "available" ? props.row.evidence : undefined;
  return (
    <p class="question-library-row-evidence" aria-label="Learning evidence">
      <Show
        when={availableEvidence()}
        fallback="More evidence is needed. This question remains ranked by relevance."
      >
        {(available) => {
          const discrimination = available().discriminationIndex;
          const discriminationText =
            discrimination === undefined
              ? ""
              : `; discrimination ${decimalNumber.format(discrimination)}`;
          return `${wholeNumber.format(available().observedCourseCount)} observed courses; ${wholeNumber.format(available().independentLearnerObservationCount)} independent Student observations; mean score ${percentage.format(available().difficultyIndex)}${discriminationText}.`;
        }}
      </Show>
    </p>
  );
}

export interface LibraryPageProps {
  readonly repository: QuestionLibraryRepository;
  readonly curation: QuestionCurationRepository;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly mayMutatePersonalCuration: boolean;
}

/** Question Library UI with the production repository injected by the route composition. */
export function LibraryPage(props: LibraryPageProps): JSX.Element {
  const [query, setQuery] = createSignal<QuestionSearchQuery>(EMPTY_QUESTION_SEARCH_QUERY);
  const [state, setState] = createSignal<QuestionSearchState>({
    kind: "loading",
    rows: [],
    aggregates: [],
    nextCursor: null,
  });
  const [scrollTop, setScrollTop] = createSignal(0);
  const [viewportHeight, setViewportHeight] = createSignal(560);
  const [rowHeightPx, setRowHeightPx] = createSignal(FALLBACK_ROW_HEIGHT_PX);
  const session = new QuestionSearchSession(props.repository, setState);

  const ready = (): Extract<QuestionSearchState, { readonly kind: "ready" }> | undefined => {
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
      | "questionType"
      | "classification"
      | "capability"
      | "questionLicense"
      | "evidence"
      | "usedInMyCourses",
  ): (() => ReadonlyArray<{ readonly value: string; readonly count: number }>) => {
    return () => aggregates().filter((aggregate) => aggregate.facet === facet);
  };
  const displayedRows = (): ReadonlyArray<QuestionSearchResult> => {
    const current = state();
    return current.kind === "empty" ? [] : current.rows;
  };
  const virtualWindow = (): Readonly<{
    readonly offset: number;
    readonly rows: ReadonlyArray<QuestionSearchResult>;
  }> =>
    questionSearchVirtualWindow(
      displayedRows(),
      scrollTop(),
      viewportHeight(),
      rowHeightPx(),
      OVERSCAN_ROWS,
    );

  function changeQuery(change: Partial<QuestionSearchQuery>): void {
    const next = { ...query(), ...change };
    setQuery(next);
    setScrollTop(0);
    void session.reset(next);
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
    void session.reset(query());
  });

  return (
    <section class="page library-page" data-route-surface="library">
      <p class="eyebrow">Shared educational content</p>
      <h1>Question library</h1>
      <p class="page-lede">Find a current published question to study, reuse, or assign.</p>
      <p>
        <A class="quiet-link" href="/curriculum">
          Browse reusable curricula
        </A>
      </p>
      <p class="sr-only" role="status" aria-live="polite">
        {state().kind === "loading" ? "Loading Question Library results." : ""}
      </p>
      <form class="question-library-controls" onSubmit={(event) => event.preventDefault()}>
        <label>
          Search published questions
          <input
            type="search"
            value={query().search}
            onInput={(event) => changeQuery({ search: event.currentTarget.value })}
            placeholder="Title or concept"
          />
        </label>
        <label>
          Question Author
          <select
            value={query().authorName ?? ""}
            onChange={(event) => changeQuery({ authorName: event.currentTarget.value || null })}
          >
            <option value="">All Question Authors</option>
            <For each={facets("authorName")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Backend
          <select
            value={query().backend ?? ""}
            onChange={(event) => changeQuery({ backend: event.currentTarget.value || null })}
          >
            <option value="">All backends</option>
            <For each={facets("backend")()}>
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
          >
            <option value="">All tags</option>
            <For each={facets("tag")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Question Type
          <select
            value={query().questionType ?? ""}
            onChange={(event) => changeQuery({ questionType: event.currentTarget.value || null })}
          >
            <option value="">All Question Types</option>
            <For each={facets("questionType")()}>
              {(facet) => (
                <option
                  value={facet.value}
                >{`${questionTypeLabel(facet.value)} (${facet.count})`}</option>
              )}
            </For>
          </select>
        </label>
        <label>
          Topic
          <select
            value={query().classification ?? ""}
            onChange={(event) => changeQuery({ classification: event.currentTarget.value || null })}
          >
            <option value="">All topics</option>
            <For each={facets("classification")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
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
          >
            <option value="">All Question Licenses</option>
            <For each={facets("questionLicense")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Evidence
          <select
            value={query().evidence ?? ""}
            onChange={(event) => changeQuery({ evidence: event.currentTarget.value || null })}
          >
            <option value="">Any evidence</option>
            <For each={facets("evidence")()}>
              {(facet) => (
                <option value={facet.value}>
                  {`${facet.value === "available" ? "Has disclosed evidence" : "Insufficient evidence"} (${facet.count})`}
                </option>
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
          >
            <option value="">Any course use</option>
            <For each={facets("usedInMyCourses")()}>
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
            onChange={(event) => changeQuery({ capability: event.currentTarget.value || null })}
          >
            <option value="">All capabilities</option>
            <For each={facets("capability")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
      </form>
      <QuestionCurationPanel
        repository={props.curation}
        pickerRepository={props.pickerRepository}
        query={query}
        applyQuery={changeQuery}
        mayMutatePersonalCuration={props.mayMutatePersonalCuration}
      />
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
          <p>Try a shorter search or choose a broader topic.</p>
        </section>
      </Show>
      <Show when={displayedRows().length > 0}>
        <div
          class="question-library-window"
          role="region"
          aria-label="Published questions"
          tabIndex={0}
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
                    <h2>{row.title}</h2>
                    <p class="question-library-row-summary">{row.summary}</p>
                    <p class="question-library-row-authors" aria-label="Question Authors">
                      Authors: {row.authorNames.join(", ")}
                    </p>
                    <p class="question-library-row-classifications card-kicker">
                      {row.classifications.join(" / ")}
                    </p>
                    <QuestionStatisticsPreview row={row} />
                    <CopyableQuestionId displayId={row.displayId} />
                    <A class="quiet-link" href={questionLink(row)}>
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
    </section>
  );
}
