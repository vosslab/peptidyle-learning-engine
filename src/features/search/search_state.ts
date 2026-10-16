import { createSignal, onCleanup, type Accessor } from "solid-js";

import type { RecordPageSize } from "../../components/record_list/record_page_controls";
import { SearchSession, type SearchDefinition, type SearchState } from "./search_session";

/** Reactive UI ownership around one plain SearchSession. */
export type SearchStateController<Query, Row, FilterCounts> = {
  readonly definition: SearchDefinition<Query, Row, FilterCounts>;
  readonly query: Accessor<Query>;
  readonly state: Accessor<SearchState<Row, FilterCounts>>;
  readonly typedText: Accessor<string>;
  readonly setTypedText: (text: string) => void;
  /** True only for nonempty text entered in this open page, never a URL starting value. */
  readonly hasTypedDraft: Accessor<boolean>;
  /** True once this page has performed a user-directed search operation. */
  readonly searchUsed: Accessor<boolean>;
  /** Advances when an accepted open or full clear replaces the current query. */
  readonly replacementRevision: () => number;
  readonly pageSize: Accessor<RecordPageSize>;
  readonly selectedIds: Accessor<ReadonlySet<string>>;
  readonly hasPrevious: Accessor<boolean>;
  readonly submit: () => Promise<void>;
  /** Applies user-directed query changes such as filters and sort. */
  readonly apply: (query: Query | ((current: Query) => Query | undefined)) => Promise<void>;
  /** Opens a starting query without turning on the leave warning. */
  readonly open: (query: Query) => Promise<void>;
  readonly setPageSize: (pageSize: RecordPageSize) => Promise<void>;
  readonly next: () => Promise<void>;
  readonly previous: () => Promise<void>;
  readonly retry: () => Promise<void>;
  /** Reloads the current page without changing the applied query or selection. */
  readonly refresh: () => Promise<void>;
  /** Returns false when a content editor keeps its work instead of clearing search. */
  readonly clear: () => Promise<boolean>;
  readonly select: (row: Row, selected: boolean) => void;
  readonly selectLoaded: () => void;
  readonly clearSelection: () => void;
  /** Removes selected stable IDs while retaining every other selected result. */
  readonly deselectIds: (ids: ReadonlyArray<string>) => void;
};

export type SearchStateOptions<Query> = {
  readonly initialQuery?: Query;
  readonly runOnMount?: boolean;
  /** Ask before a query operation clears content-owned editors or selected rows. */
  readonly beforeQueryChange?: () => Promise<boolean>;
};

/**
 * Connects the framework-neutral request session to one open Solid view.
 * Typing deliberately changes only local text; submit is the single search boundary.
 */
export function createSearchState<Query, Row, FilterCounts>(
  definition: SearchDefinition<Query, Row, FilterCounts>,
  options: SearchStateOptions<Query> = {},
): SearchStateController<Query, Row, FilterCounts> {
  const [query, setQuery] = createSignal(options.initialQuery ?? definition.initialQuery);
  const [state, setState] = createSignal<SearchState<Row, FilterCounts>>({
    kind: "initial",
    rows: [],
    filterCounts: undefined,
    nextCursor: null,
  });
  const [typedText, setTypedTextSignal] = createSignal(definition.getText(query()));
  const [typedByUser, setTypedByUser] = createSignal(false);
  const [searchUsed, setSearchUsed] = createSignal(false);
  const hasTypedDraft = (): boolean => typedByUser() && typedText().length > 0;
  const [pageSize, setPageSizeSignal] = createSignal<RecordPageSize>(50);
  const [selectedIds, setSelectedIds] = createSignal<ReadonlySet<string>>(new Set());
  const [hasPrevious, setHasPrevious] = createSignal(false);
  let disposed = false;
  let pendingUpdaterRevision = 0;
  const session = new SearchSession(definition, (next) => {
    setState(next);
    setQuery(() => session.query);
    setPageSizeSignal(session.pageSize);
    setSelectedIds(new Set(session.selectedIds));
    setHasPrevious(session.hasPrevious);
  });

  async function open(nextQuery: Query): Promise<void> {
    if (options.beforeQueryChange !== undefined && !(await options.beforeQueryChange())) return;
    if (disposed) return;
    pendingUpdaterRevision++;
    setTypedTextSignal(definition.getText(nextQuery));
    setTypedByUser(false);
    setSearchUsed(false);
    await session.apply(nextQuery);
  }

  async function apply(nextQuery: Query | ((current: Query) => Query | undefined)): Promise<void> {
    const updaterRevision = pendingUpdaterRevision;
    if (options.beforeQueryChange !== undefined && !(await options.beforeQueryChange())) return;
    if (disposed) return;
    if (typeof nextQuery === "function" && updaterRevision !== pendingUpdaterRevision) return;
    const current = session.query;
    const resolvedQuery =
      typeof nextQuery === "function"
        ? (nextQuery as (current: Query) => Query | undefined)(current)
        : nextQuery;
    if (resolvedQuery === undefined) return;
    const appliedText = definition.getText(current);
    const cleanedQuery = definition.cleanup(resolvedQuery);
    const nextText = definition.getText(cleanedQuery);
    if (nextText !== appliedText) {
      setTypedTextSignal(nextText);
      setTypedByUser(false);
    }
    setSearchUsed(true);
    await session.apply(cleanedQuery);
  }

  async function submit(): Promise<void> {
    const submittedText = typedText();
    await apply((current) => definition.setText(current, submittedText));
  }

  function setTypedText(text: string): void {
    setTypedTextSignal(text);
    setTypedByUser(true);
  }

  async function clearNow(): Promise<void> {
    pendingUpdaterRevision++;
    const startingQuery = definition.cleanup(definition.initialQuery);
    setTypedTextSignal(definition.getText(startingQuery));
    setTypedByUser(false);
    setSearchUsed(false);
    session.reset();
    setQuery(() => session.query);
    setPageSizeSignal(session.pageSize);
    setSelectedIds(new Set<string>());
    setHasPrevious(false);
    if (definition.isEligible(startingQuery)) await session.apply(startingQuery);
  }

  function clear(): Promise<boolean> {
    if (options.beforeQueryChange === undefined) {
      if (disposed) return Promise.resolve(false);
      return clearNow().then(() => true);
    }
    return options.beforeQueryChange().then((accepted) => {
      if (!accepted || disposed) return false;
      return clearNow().then(() => true);
    });
  }

  if (options.runOnMount === true) void open(query());
  onCleanup(() => {
    disposed = true;
    session.dispose();
  });

  return {
    definition,
    query,
    state,
    typedText,
    setTypedText,
    hasTypedDraft,
    searchUsed,
    replacementRevision: (): number => pendingUpdaterRevision,
    pageSize,
    selectedIds,
    hasPrevious,
    submit,
    apply,
    open,
    setPageSize: async (size): Promise<void> => {
      setSearchUsed(true);
      await session.setPageSize(size);
    },
    next: async (): Promise<void> => {
      setSearchUsed(true);
      await session.next();
    },
    previous: async (): Promise<void> => {
      setSearchUsed(true);
      await session.previous();
    },
    retry: async (): Promise<void> => {
      setSearchUsed(true);
      await session.retry();
    },
    refresh: (): Promise<void> => session.refresh(),
    clear,
    select: (row, selected): void => {
      session.select(definition.rowId(row), selected);
      setSelectedIds(new Set(session.selectedIds));
    },
    selectLoaded: (): void => {
      session.selectLoaded();
      setSelectedIds(new Set(session.selectedIds));
    },
    clearSelection: (): void => {
      session.clearSelection();
      setSelectedIds(new Set<string>());
    },
    deselectIds: (ids): void => {
      session.deselectIds(ids);
      setSelectedIds(new Set(session.selectedIds));
    },
  };
}
