// library_page.tsx - injected catalog browse surface; route wiring follows the server contract.

import { A } from "@solidjs/router";
import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import { CopyableQuestionId } from "../components/copyable_question_id";
import { ProblemCurationPanel } from "../features/problem_curation/problem_curation_panel";
import type { ProblemCurationRepository } from "../features/problem_curation/problem_curation_model";
import type { ProblemPickerSourceRepository } from "../features/problem_picker";
import "./library_page.css";
import {
  EMPTY_CATALOG_QUERY,
  CatalogBrowseSession,
  catalogVirtualWindow,
  type CatalogBrowseQuery,
  type CatalogBrowseRepository,
  type CatalogBrowseRow,
  type CatalogBrowseState,
} from "./library_page_model";

/* Each virtual row reserves room for a title, two-line summary, byline, and taxonomy.
 * Keep this fallback aligned with --ple-catalog-row-block-size in src/style.css. */
const FALLBACK_ROW_HEIGHT_PX = 164;
const OVERSCAN_ROWS = 5;
const percentage = new Intl.NumberFormat("en-US", { style: "percent", maximumFractionDigits: 0 });
const wholeNumber = new Intl.NumberFormat("en-US");
const decimalNumber = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });

function catalogLink(row: CatalogBrowseRow): string {
  return `/library/${encodeURIComponent(row.displayId)}`;
}

function responseFamilyLabel(value: string): string {
  const labels: Readonly<Record<string, string>> = {
    numeric: "Numeric",
    multipleChoice: "Multiple choice",
    shortText: "Short text",
    multiBlank: "Multiple blanks",
    matching: "Matching",
    ordering: "Ordering",
    hotspot: "Hotspot",
    fileUpload: "File upload",
    externalTool: "External tool",
  };
  return labels[value] ?? value;
}

function backendLabel(value: string): string {
  const labels: Readonly<Record<string, string>> = {
    native: "Native",
    webwork: "WeBWorK",
    qti: "QTI",
    h5p: "H5P",
    imathas: "IMathAS",
  };
  return labels[value] ?? value;
}

function CatalogEvidencePreview(props: { readonly row: CatalogBrowseRow }): JSX.Element {
  const availableEvidence = ():
    Extract<CatalogBrowseRow["evidence"], { readonly state: "available" }> | undefined =>
    props.row.evidence.state === "available" ? props.row.evidence : undefined;
  return (
    <p class="catalog-row-evidence" aria-label="Learning evidence">
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
  readonly repository: CatalogBrowseRepository;
  readonly curation: ProblemCurationRepository;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly mayMutatePersonalCuration: boolean;
}

/** Catalog UI with the production repository injected by the route composition. */
export function LibraryPage(props: LibraryPageProps): JSX.Element {
  const [query, setQuery] = createSignal<CatalogBrowseQuery>(EMPTY_CATALOG_QUERY);
  const [state, setState] = createSignal<CatalogBrowseState>({
    kind: "loading",
    rows: [],
    aggregates: [],
    nextCursor: null,
  });
  const [scrollTop, setScrollTop] = createSignal(0);
  const [viewportHeight, setViewportHeight] = createSignal(560);
  const [rowHeightPx, setRowHeightPx] = createSignal(FALLBACK_ROW_HEIGHT_PX);
  const session = new CatalogBrowseSession(props.repository, setState);

  const ready = (): Extract<CatalogBrowseState, { readonly kind: "ready" }> | undefined => {
    const current = state();
    return current.kind === "ready" ? current : undefined;
  };
  const aggregates = (): ReadonlyArray<{
    readonly group: string;
    readonly value: string;
    readonly count: number;
  }> => {
    const current = state();
    return current.aggregates;
  };
  const facets = (
    group:
      | "byline"
      | "backend"
      | "tag"
      | "responseFamily"
      | "taxonomy"
      | "capability"
      | "license"
      | "evidence"
      | "usedInMyCourses",
  ): (() => ReadonlyArray<{ readonly value: string; readonly count: number }>) => {
    return () => aggregates().filter((aggregate) => aggregate.group === group);
  };
  const displayedRows = (): ReadonlyArray<CatalogBrowseRow> => {
    const current = state();
    return current.kind === "empty" ? [] : current.rows;
  };
  const virtualWindow = (): Readonly<{
    readonly offset: number;
    readonly rows: ReadonlyArray<CatalogBrowseRow>;
  }> =>
    catalogVirtualWindow(
      displayedRows(),
      scrollTop(),
      viewportHeight(),
      rowHeightPx(),
      OVERSCAN_ROWS,
    );

  function changeQuery(change: Partial<CatalogBrowseQuery>): void {
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
        getComputedStyle(document.documentElement).getPropertyValue("--ple-catalog-row-block-size"),
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
        {state().kind === "loading" ? "Loading catalog results." : ""}
      </p>
      <form class="catalog-controls" onSubmit={(event) => event.preventDefault()}>
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
          Byline
          <select
            value={query().byline ?? ""}
            onChange={(event) => changeQuery({ byline: event.currentTarget.value || null })}
          >
            <option value="">All bylines</option>
            <For each={facets("byline")()}>
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
          Response family
          <select
            value={query().responseFamily ?? ""}
            onChange={(event) => changeQuery({ responseFamily: event.currentTarget.value || null })}
          >
            <option value="">All response families</option>
            <For each={facets("responseFamily")()}>
              {(facet) => (
                <option
                  value={facet.value}
                >{`${responseFamilyLabel(facet.value)} (${facet.count})`}</option>
              )}
            </For>
          </select>
        </label>
        <label>
          Topic
          <select
            value={query().taxonomy ?? ""}
            onChange={(event) => changeQuery({ taxonomy: event.currentTarget.value || null })}
          >
            <option value="">All topics</option>
            <For each={facets("taxonomy")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          License
          <select
            value={query().license ?? ""}
            onChange={(event) => changeQuery({ license: event.currentTarget.value || null })}
          >
            <option value="">All licenses</option>
            <For each={facets("license")()}>
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
      <ProblemCurationPanel
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
          class="catalog-window"
          role="region"
          aria-label="Published questions"
          tabIndex={0}
          onScroll={handleScroll}
          style={`--ple-catalog-loaded-block-size:${displayedRows().length * rowHeightPx()}px`}
        >
          <div
            style={{
              height: `${displayedRows().length * rowHeightPx()}px`,
              position: "relative",
            }}
          >
            <div class="catalog-window-slice" style={{ top: `${virtualWindow().offset}px` }}>
              <For each={virtualWindow().rows}>
                {(row) => (
                  <article class="catalog-row" style={{ height: `${rowHeightPx()}px` }}>
                    <h2>{row.title}</h2>
                    <p class="catalog-row-summary">{row.summary}</p>
                    <p class="catalog-row-byline" aria-label="Published by">
                      By {row.byline.join(", ")}
                    </p>
                    <p class="catalog-row-taxonomy card-kicker">{row.taxonomy.join(" / ")}</p>
                    <CatalogEvidencePreview row={row} />
                    <CopyableQuestionId displayId={row.displayId} />
                    <A class="quiet-link" href={catalogLink(row)}>
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
