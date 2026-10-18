import {
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  untrack,
  type Accessor,
} from "solid-js";

import { RECORD_PAGE_SIZES, type RecordPageSize } from "../data_sheet/data_sheet_page_sizes";
import type {
  SearchDefinition,
  SearchPage,
  SearchPageRequest,
  SearchState,
} from "./search_definition";

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
  readonly pageNumber: Accessor<number>;
  readonly totalCount: Accessor<number>;
  readonly totalPages: Accessor<number>;
  readonly selectedIds: Accessor<ReadonlySet<string>>;
  readonly hasPrevious: Accessor<boolean>;
  readonly submit: () => Promise<void>;
  /** Applies keyword/filter changes and clears selection for the changed result set. */
  readonly apply: (query: Query | ((current: Query) => Query | undefined)) => Promise<void>;
  /** Applies an ordering change as page one while retaining selection by stable row ID. */
  readonly sort: (query: Query | ((current: Query) => Query | undefined)) => Promise<void>;
  /** Opens a starting query without turning on the leave warning. */
  readonly open: (query: Query) => Promise<void>;
  readonly setPageSize: (pageSize: RecordPageSize) => Promise<void>;
  readonly goToPage: (pageNumber: number) => Promise<void>;
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

export type SearchSourceLifecycle<Row> = {
  /** Undefined means the source has not delivered rows yet. */
  readonly rows: Accessor<ReadonlyArray<Row> | undefined>;
  readonly loading?: Accessor<boolean>;
  readonly error?: Accessor<unknown>;
  readonly refetch?: () => unknown;
};

export type SearchStateOptions<Query, Row> = {
  readonly initialQuery?: Query;
  readonly runOnMount?: boolean;
  /** Ask before a query operation clears content-owned editors or selected rows. */
  readonly beforeQueryChange?: () => Promise<boolean>;
  /** Source status remains owned by the data resource and is read by this controller. */
  readonly source?: SearchSourceLifecycle<Row>;
};

type SearchRequest<Query> = {
  readonly query: Query;
  readonly page: SearchPageRequest;
  /** Reuse rows while refreshing or moving between numbered pages. */
  readonly retainPage: boolean;
};

const DEFAULT_PAGE_SIZE: RecordPageSize = 50;

function pageCount(totalCount: number, pageSize: RecordPageSize): number {
  return Math.max(1, Math.ceil(totalCount / pageSize));
}

function initialState<Row>(pageSize: RecordPageSize): SearchState<Row, never> {
  return {
    kind: "initial",
    rows: [],
    filterCounts: undefined,
    pageNumber: 1,
    pageSize,
    totalCount: 0,
  };
}

function isValidPageNumber(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 1;
}

function isValidTotalCount(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0;
}

/** One Solid owner for a numbered query, its requests, selection, and visible lifecycle. */
export function createSearchState<Query, Row, FilterCounts>(
  definition: SearchDefinition<Query, Row, FilterCounts>,
  options: SearchStateOptions<Query, Row> = {},
): SearchStateController<Query, Row, FilterCounts> {
  const startingQuery = definition.cleanup(options.initialQuery ?? definition.initialQuery);
  const [query, setQuery] = createSignal<Query>(startingQuery);
  const [baseState, setBaseState] = createSignal<SearchState<Row, FilterCounts>>(
    initialState<Row>(DEFAULT_PAGE_SIZE),
  );
  const state = createMemo<SearchState<Row, FilterCounts>>(() => {
    const current = baseState();
    const sourceError = options.source?.error?.();
    if (sourceError !== undefined) return { ...current, kind: "error", error: sourceError };
    if (options.source?.loading?.()) return { ...current, kind: "loading" };
    return current;
  });
  const [typedText, setTypedTextSignal] = createSignal(definition.getText(startingQuery));
  const [typedByUser, setTypedByUser] = createSignal(false);
  const [searchUsed, setSearchUsed] = createSignal(false);
  const hasTypedDraft = (): boolean => typedByUser() && typedText().length > 0;
  const pageSize = (): RecordPageSize => baseState().pageSize;
  const pageNumber = (): number => baseState().pageNumber;
  const totalCount = (): number => baseState().totalCount;
  const totalPages = (): number => pageCount(totalCount(), pageSize());
  const [selectedIds, setSelectedIds] = createSignal<ReadonlySet<string>>(new Set());
  const hasPrevious = (): boolean => pageNumber() > 1;
  let disposed = false;
  let requestNumber = 0;
  let failedRequest: SearchRequest<Query> | undefined;
  let pendingUpdaterRevision = 0;
  let sourceDelivered = false;

  function validatePage(page: SearchPage<Row, FilterCounts>, requested: SearchPageRequest): void {
    if (!isValidPageNumber(page.pageNumber)) throw new Error("Invalid resolved page number.");
    if (!isValidTotalCount(page.totalCount)) throw new Error("Invalid total result count.");
    if (page.pageSize !== requested.pageSize) throw new Error("Unexpected resolved page size.");
    if (page.totalCount === 0 && page.pageNumber !== 1)
      throw new Error("An empty result set must resolve to page one.");
    if (page.totalCount > 0 && page.items.length === 0)
      throw new Error("A resolved page for matching results must contain a row.");
    if (page.pageNumber > pageCount(page.totalCount, page.pageSize))
      throw new Error("Resolved page number exceeds the result count.");
    const remainingCount = page.totalCount - (page.pageNumber - 1) * page.pageSize;
    if (page.items.length > Math.min(page.pageSize, remainingCount))
      throw new Error("Page contains more rows than remain in the result set.");
  }

  async function load(request: SearchRequest<Query>): Promise<void> {
    const currentRequest = ++requestNumber;
    failedRequest = undefined;
    const previous = baseState();
    const retainedRows = request.retainPage ? previous.rows : [];
    const retainedFilterCounts = request.retainPage ? previous.filterCounts : undefined;
    const retainedTotalCount = request.retainPage ? previous.totalCount : 0;
    const retainedPageNumber = request.retainPage ? previous.pageNumber : request.page.pageNumber;
    setBaseState({
      kind: "loading",
      rows: retainedRows,
      filterCounts: retainedFilterCounts,
      pageNumber: retainedPageNumber,
      pageSize: request.page.pageSize,
      totalCount: retainedTotalCount,
    });
    let page: SearchPage<Row, FilterCounts>;
    try {
      page = await definition.fetchPage(request.query, request.page);
      validatePage(page, request.page);
    } catch (error: unknown) {
      if (disposed || currentRequest !== requestNumber) return;
      failedRequest = request;
      setBaseState({
        kind: "error",
        rows: retainedRows,
        filterCounts: retainedFilterCounts,
        pageNumber: retainedPageNumber,
        pageSize: request.page.pageSize,
        totalCount: retainedTotalCount,
        error,
      });
      return;
    }
    if (disposed || currentRequest !== requestNumber) return;
    setQuery(() => request.query);
    setBaseState(
      page.items.length === 0
        ? {
            kind: "empty",
            rows: [],
            filterCounts: page.filterCounts,
            pageNumber: page.pageNumber,
            pageSize: page.pageSize,
            totalCount: page.totalCount,
          }
        : {
            kind: "ready",
            rows: page.items,
            filterCounts: page.filterCounts,
            pageNumber: page.pageNumber,
            pageSize: page.pageSize,
            totalCount: page.totalCount,
          },
    );
  }

  function clearSelection(): void {
    setSelectedIds(new Set<string>());
  }

  async function startAtFirstPage(nextQuery: Query, clearSelected: boolean): Promise<void> {
    if (disposed) return;
    const cleanedQuery = definition.cleanup(nextQuery);
    setQuery(() => cleanedQuery);
    if (clearSelected) clearSelection();
    if (!definition.isEligible(cleanedQuery)) {
      requestNumber++;
      failedRequest = undefined;
      setBaseState(initialState<Row>(pageSize()));
      return;
    }
    await load({
      query: cleanedQuery,
      page: { pageNumber: 1, pageSize: pageSize() },
      retainPage: false,
    });
  }

  async function open(nextQuery: Query): Promise<void> {
    const replacementRevision = pendingUpdaterRevision;
    if (options.beforeQueryChange !== undefined && !(await options.beforeQueryChange())) return;
    if (disposed || replacementRevision !== pendingUpdaterRevision) return;
    pendingUpdaterRevision++;
    setTypedTextSignal(definition.getText(nextQuery));
    setTypedByUser(false);
    setSearchUsed(false);
    await startAtFirstPage(nextQuery, true);
  }

  async function apply(nextQuery: Query | ((current: Query) => Query | undefined)): Promise<void> {
    const updaterRevision = pendingUpdaterRevision;
    if (options.beforeQueryChange !== undefined && !(await options.beforeQueryChange())) return;
    if (disposed) return;
    if (updaterRevision !== pendingUpdaterRevision) return;
    const current = query();
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
    await startAtFirstPage(cleanedQuery, true);
  }

  async function sort(nextQuery: Query | ((current: Query) => Query | undefined)): Promise<void> {
    if (disposed) return;
    const current = query();
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
    await startAtFirstPage(cleanedQuery, false);
  }

  async function submit(): Promise<void> {
    const submittedText = typedText();
    await apply((current) => definition.setText(current, submittedText));
  }

  function setTypedText(text: string): void {
    setTypedTextSignal(text);
    setTypedByUser(true);
  }

  async function setPageSize(size: RecordPageSize): Promise<void> {
    setSearchUsed(true);
    if (
      disposed ||
      !RECORD_PAGE_SIZES.includes(size) ||
      size > (definition.maximumPageSize ?? RECORD_PAGE_SIZES[RECORD_PAGE_SIZES.length - 1]!) ||
      size === pageSize()
    )
      return;
    const firstRecordOffset = (pageNumber() - 1) * pageSize();
    const requestedPageNumber = Math.floor(firstRecordOffset / size) + 1;
    if (baseState().kind === "initial") {
      setBaseState(initialState<Row>(size));
      return;
    }
    await load({
      query: query(),
      page: { pageNumber: requestedPageNumber, pageSize: size },
      retainPage: true,
    });
  }

  async function goToPage(page: number): Promise<void> {
    setSearchUsed(true);
    if (disposed || baseState().kind === "initial" || !isValidPageNumber(page)) return;
    await load({
      query: query(),
      page: { pageNumber: page, pageSize: pageSize() },
      retainPage: true,
    });
  }

  async function next(): Promise<void> {
    setSearchUsed(true);
    if (disposed || baseState().kind !== "ready" || pageNumber() >= totalPages()) return;
    await goToPage(pageNumber() + 1);
  }

  async function previous(): Promise<void> {
    setSearchUsed(true);
    if (disposed || baseState().kind === "loading" || !hasPrevious()) return;
    await goToPage(pageNumber() - 1);
  }

  async function refresh(): Promise<void> {
    if (disposed) return;
    if (options.source?.refetch !== undefined) {
      await options.source.refetch();
      return;
    }
    await reloadCurrentPage();
  }

  async function reloadCurrentPage(): Promise<void> {
    if (baseState().kind === "initial") return;
    await load({
      query: query(),
      page: { pageNumber: pageNumber(), pageSize: pageSize() },
      retainPage: true,
    });
  }

  async function retry(): Promise<void> {
    setSearchUsed(true);
    if (disposed) return;
    if (options.source?.error?.() !== undefined && options.source.refetch !== undefined) {
      await options.source.refetch();
      return;
    }
    if (failedRequest !== undefined) await load(failedRequest);
  }

  async function clearNow(): Promise<void> {
    pendingUpdaterRevision++;
    const resetQuery = definition.cleanup(definition.initialQuery);
    setTypedTextSignal(definition.getText(resetQuery));
    setTypedByUser(false);
    setSearchUsed(false);
    requestNumber++;
    failedRequest = undefined;
    setQuery(() => resetQuery);
    clearSelection();
    setBaseState(initialState<Row>(pageSize()));
    if (definition.isEligible(resetQuery)) await startAtFirstPage(resetQuery, true);
  }

  async function clear(): Promise<boolean> {
    const replacementRevision = pendingUpdaterRevision;
    if (options.beforeQueryChange !== undefined && !(await options.beforeQueryChange()))
      return false;
    if (disposed || replacementRevision !== pendingUpdaterRevision) return false;
    await clearNow();
    return !disposed && pendingUpdaterRevision === replacementRevision + 1;
  }

  function selectId(id: string, selected: boolean): void {
    if (disposed) return;
    const nextIds = new Set(selectedIds());
    if (!selected) nextIds.delete(id);
    else if (nextIds.has(id) || nextIds.size < (definition.selection?.maximum ?? 0))
      nextIds.add(id);
    setSelectedIds(nextIds);
  }

  function deselectIds(ids: ReadonlyArray<string>): void {
    if (disposed) return;
    const nextIds = new Set(selectedIds());
    for (const id of ids) nextIds.delete(id);
    setSelectedIds(nextIds);
  }

  createEffect(() => {
    if (options.source?.error?.() !== undefined) return;
    const rows = options.source?.rows();
    if (rows === undefined) return;
    untrack(() => {
      const availableIds = new Set(rows.map(definition.rowId));
      deselectIds([...selectedIds()].filter((id) => !availableIds.has(id)));
      if (!sourceDelivered && baseState().kind === "initial") {
        sourceDelivered = true;
        const draft = hasTypedDraft() ? typedText() : undefined;
        void open(query());
        if (draft !== undefined) setTypedText(draft);
      } else {
        sourceDelivered = true;
        void reloadCurrentPage();
      }
    });
  });

  if (options.runOnMount === true) void open(query());
  onCleanup(() => {
    disposed = true;
    requestNumber++;
    failedRequest = undefined;
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
    pageNumber,
    totalCount,
    totalPages,
    selectedIds,
    hasPrevious,
    submit,
    apply,
    sort,
    open,
    setPageSize,
    goToPage,
    next,
    previous,
    retry,
    refresh,
    clear,
    select: (row, selected): void => selectId(definition.rowId(row), selected),
    selectLoaded: (): void => {
      if (disposed) return;
      for (const row of baseState().rows) selectId(definition.rowId(row), true);
    },
    clearSelection,
    deselectIds,
  };
}
