// library_page.tsx - injected catalog browse surface; route wiring follows the server contract.

import { A } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import { CopyableProblemId } from "../components/copyable_problem_id";
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

const ROW_HEIGHT_PX = 152;
const OVERSCAN_ROWS = 5;

function catalogLink(row: CatalogBrowseRow): string {
  return `/library/${encodeURIComponent(row.problemId)}/versions/${encodeURIComponent(row.versionId)}`;
}

export interface LibraryPageProps {
  readonly repository: CatalogBrowseRepository;
}

/** Mock-ready catalog UI. Production routes inject the generated-client repository after P0 exists. */
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
    group: "taxonomy" | "capability" | "license" | "statistic",
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
      ROW_HEIGHT_PX,
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
      target.scrollTop + target.clientHeight >= target.scrollHeight - ROW_HEIGHT_PX * 3
    ) {
      void session.loadNext();
    }
  }

  onMount(() => void session.reset(query()));

  return (
    <section class="page library-page" data-route-surface="library">
      <p class="eyebrow">Shared educational content</p>
      <h1>Problem library</h1>
      <p class="page-lede">Find an immutable published version to study, reuse, or assign.</p>
      <p class="sr-only" role="status" aria-live="polite">
        {state().kind === "loading" ? "Loading catalog results." : ""}
      </p>
      <form class="catalog-controls" onSubmit={(event) => event.preventDefault()}>
        <label>
          Search published problems
          <input
            type="search"
            value={query().search}
            onInput={(event) => changeQuery({ search: event.currentTarget.value })}
            placeholder="Title or concept"
          />
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
          Statistic availability
          <select
            value={query().statistic ?? ""}
            onChange={(event) => changeQuery({ statistic: event.currentTarget.value || null })}
          >
            <option value="">All availability</option>
            <For each={facets("statistic")()}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
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
        <section class="empty-state" aria-label="No matching published problems">
          <h2>No published problems match these filters</h2>
          <p>Try a shorter search or choose a broader topic.</p>
        </section>
      </Show>
      <Show when={displayedRows().length > 0}>
        <div
          class="catalog-window"
          role="region"
          aria-label="Published problems"
          tabIndex={0}
          onScroll={handleScroll}
        >
          <div
            style={{
              height: `${displayedRows().length * ROW_HEIGHT_PX}px`,
              position: "relative",
            }}
          >
            <div class="catalog-window-slice" style={{ top: `${virtualWindow().offset}px` }}>
              <For each={virtualWindow().rows}>
                {(row) => (
                  <article class="catalog-row" style={{ height: `${ROW_HEIGHT_PX}px` }}>
                    <h2>{row.title}</h2>
                    <p>{row.summary}</p>
                    <p class="card-kicker">{row.taxonomy.join(" * ")}</p>
                    <CopyableProblemId displayId={row.displayId} />
                    <A class="quiet-link" href={catalogLink(row)}>
                      Open immutable version
                    </A>
                  </article>
                )}
              </For>
            </div>
          </div>
          <Show when={state().kind === "loading"}>
            <p class="loading-state" role="status">
              Loading more published problems...
            </p>
          </Show>
        </div>
      </Show>
      <Show when={state().kind === "loading" && displayedRows().length === 0}>
        <p class="loading-state" role="status">
          Loading published problems...
        </p>
      </Show>
    </section>
  );
}
