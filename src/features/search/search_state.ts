import { createSignal, onCleanup, type Accessor, type Setter } from "solid-js";

import type { RecordPageSize } from "../../components/record_list/record_page_controls";
import type { SearchResultDisplayMode } from "../../components/search_result_display";
import { SearchSession, type SearchDefinition, type SearchState } from "./search_session";

/** Reactive UI ownership around one plain SearchSession. */
export type SearchStateController<Query, Row, FilterCounts, Sort extends string = never> = {
  readonly definition: SearchDefinition<Query, Row, FilterCounts, Sort>;
  readonly query: Accessor<Query>;
  readonly state: Accessor<SearchState<Row, FilterCounts>>;
  readonly typedText: Accessor<string>;
  readonly setTypedText: (text: string) => void;
  /** True only for nonempty text entered in this open page, never a URL starting value. */
  readonly hasTypedDraft: Accessor<boolean>;
  /** True once this page has performed a user-directed search operation. */
  readonly searchUsed: Accessor<boolean>;
  readonly pageSize: Accessor<RecordPageSize>;
  readonly selectedIds: Accessor<ReadonlySet<string>>;
  readonly hasPrevious: Accessor<boolean>;
  readonly display: Accessor<SearchResultDisplayMode>;
  readonly setDisplay: Setter<SearchResultDisplayMode>;
  readonly submit: () => Promise<void>;
  /** Applies user-directed query changes such as filters and sort. */
  readonly apply: (query: Query) => Promise<void>;
  /** Opens a starting query without turning on the leave warning. */
  readonly open: (query: Query) => Promise<void>;
  readonly setPageSize: (pageSize: RecordPageSize) => Promise<void>;
  readonly next: () => Promise<void>;
  readonly previous: () => Promise<void>;
  readonly retry: () => Promise<void>;
  /** Reloads the current page without changing the applied query or selection. */
  readonly refresh: () => Promise<void>;
  readonly clear: () => void;
  readonly select: (row: Row, selected: boolean) => void;
  readonly selectLoaded: () => void;
  readonly clearSelection: () => void;
  /** Removes selected stable IDs while retaining every other selected result. */
  readonly deselectIds: (ids: ReadonlyArray<string>) => void;
};

export type SearchStateOptions<Query> = {
  readonly initialQuery?: Query;
  readonly runOnMount?: boolean;
};

/**
 * Connects the framework-neutral request session to one open Solid view.
 * Typing deliberately changes only local text; submit is the single search boundary.
 */
export function createSearchState<Query, Row, FilterCounts, Sort extends string = never>(
  definition: SearchDefinition<Query, Row, FilterCounts, Sort>,
  options: SearchStateOptions<Query> = {},
): SearchStateController<Query, Row, FilterCounts, Sort> {
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
  const [display, setDisplay] = createSignal<SearchResultDisplayMode>("compact");
  const session = new SearchSession(definition, (next) => {
    setState(next);
    setQuery(() => session.query);
    setPageSizeSignal(session.pageSize);
    setSelectedIds(new Set(session.selectedIds));
    setHasPrevious(session.hasPrevious);
  });

  async function open(nextQuery: Query): Promise<void> {
    setTypedTextSignal(definition.getText(nextQuery));
    setTypedByUser(false);
    setSearchUsed(false);
    await session.apply(nextQuery);
  }

  async function apply(nextQuery: Query): Promise<void> {
    setSearchUsed(true);
    await session.apply(nextQuery);
  }

  async function submit(): Promise<void> {
    await apply(definition.setText(query(), typedText()));
  }

  function setTypedText(text: string): void {
    setTypedTextSignal(text);
    setTypedByUser(true);
  }

  function clear(): void {
    setTypedTextSignal("");
    setTypedByUser(false);
    setSearchUsed(false);
    session.reset();
    setQuery(() => definition.initialQuery);
    setPageSizeSignal(session.pageSize);
    setSelectedIds(new Set<string>());
    setHasPrevious(false);
  }

  if (options.runOnMount === true) void open(query());
  onCleanup(() => session.dispose());

  return {
    definition,
    query,
    state,
    typedText,
    setTypedText,
    hasTypedDraft,
    searchUsed,
    pageSize,
    selectedIds,
    hasPrevious,
    display,
    setDisplay,
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
