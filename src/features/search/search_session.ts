import type { RecordContent } from "../../components/record_list/record_list";
import type { RecordPageSize } from "../../components/record_list/record_page_controls";
import type { RecordSortOption } from "../../components/record_list/record_sort_control";

/** One server page for a content-specific search definition. */
export type SearchPage<Row, FilterCounts> = {
  readonly items: ReadonlyArray<Row>;
  readonly nextCursor: string | null;
  /** Counts describe the exact query that produced this page. */
  readonly filterCounts: FilterCounts;
};

/** A removable filter chip supplied by a content-specific search definition. */
export type SearchAppliedFilter<Query> = {
  readonly id: string;
  readonly label: string;
  readonly clear: (query: Query) => Query;
};

/** A content-specific server order that the shared controls can present. */
export type SearchSort<Query, Value extends string> = {
  readonly label: string;
  readonly options: ReadonlyArray<RecordSortOption<Value>>;
  readonly get: (query: Query) => Value;
  readonly set: (query: Query, value: Value) => Query;
};

/**
 * The small content-owned boundary around the reusable search machinery.
 *
 * Query cleanup and page decoding belong to the definition, before values reach this session.
 * The session only owns one page, its cursor stack, request ordering, and row-ID selection.
 */
export type SearchDefinition<Query, Row, FilterCounts, Sort extends string = never> = {
  readonly initialQuery: Query;
  readonly cleanup: (query: Query) => Query;
  readonly getText: (query: Query) => string;
  readonly setText: (query: Query, text: string) => Query;
  readonly fetchPage: (
    query: Query,
    cursor: string | null,
    pageSize: RecordPageSize,
  ) => Promise<SearchPage<Row, FilterCounts>>;
  readonly rowId: (row: Row) => string;
  readonly content: (row: Row) => RecordContent;
  readonly sort?: SearchSort<Query, Sort>;
  readonly appliedFilters?: (query: Query) => ReadonlyArray<SearchAppliedFilter<Query>>;
  readonly selection?: { readonly maximum: number };
};

export type SearchState<Row, FilterCounts> =
  | {
      readonly kind: "initial";
      readonly rows: ReadonlyArray<Row>;
      readonly filterCounts: undefined;
      readonly nextCursor: null;
    }
  | {
      readonly kind: "loading";
      readonly rows: ReadonlyArray<Row>;
      readonly filterCounts: FilterCounts | undefined;
      readonly nextCursor: string | null;
    }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<Row>;
      readonly filterCounts: FilterCounts;
      readonly nextCursor: string | null;
    }
  | {
      readonly kind: "empty";
      readonly rows: readonly [];
      readonly filterCounts: FilterCounts;
      readonly nextCursor: null;
    }
  | {
      readonly kind: "error";
      readonly rows: ReadonlyArray<Row>;
      readonly filterCounts: FilterCounts | undefined;
      readonly nextCursor: string | null;
      readonly error: unknown;
    };

type SearchRequest<Query> = {
  readonly query: Query;
  readonly cursor: string | null;
  readonly previousCursors: ReadonlyArray<string | null>;
  readonly pageSize: RecordPageSize;
  /** Reuse the visible page while moving between known cursor pages or refreshing it. */
  readonly retainPage: boolean;
};

const DEFAULT_PAGE_SIZE: RecordPageSize = 50;

/**
 * Plain TypeScript state for one shared search. It does not depend on a UI runtime.
 *
 * A later request always wins. Starting a replacement search clears all query-dependent
 * information immediately, while paging and retry retain the already verified page.
 */
export class SearchSession<Query, Row, FilterCounts, Sort extends string = never> {
  #requestNumber = 0;
  #disposed = false;
  #query: Query;
  #pageSize: RecordPageSize = DEFAULT_PAGE_SIZE;
  #currentCursor: string | null = null;
  #previousCursors: ReadonlyArray<string | null> = [];
  #failedRequest: SearchRequest<Query> | undefined;
  #selectedIds = new Set<string>();
  #state: SearchState<Row, FilterCounts> = {
    kind: "initial",
    rows: [],
    filterCounts: undefined,
    nextCursor: null,
  };

  public constructor(
    private readonly definition: SearchDefinition<Query, Row, FilterCounts, Sort>,
    private readonly publish: (state: SearchState<Row, FilterCounts>) => void = () => {},
  ) {
    this.#query = definition.cleanup(definition.initialQuery);
  }

  public get state(): SearchState<Row, FilterCounts> {
    return this.#state;
  }

  public get query(): Query {
    return this.#query;
  }

  public get pageSize(): RecordPageSize {
    return this.#pageSize;
  }

  public get selectedIds(): ReadonlySet<string> {
    return this.#selectedIds;
  }

  public get hasPrevious(): boolean {
    return this.#previousCursors.length > 0;
  }

  public get loading(): boolean {
    return this.#state.kind === "loading";
  }

  /** Applies a cleaned query as page one and clears selection for the changed result set. */
  public async apply(query: Query): Promise<void> {
    if (this.#disposed) return;
    this.#query = this.definition.cleanup(query);
    this.#currentCursor = null;
    this.#previousCursors = [];
    this.clearSelection();
    await this.load({
      query: this.#query,
      cursor: null,
      previousCursors: [],
      pageSize: this.#pageSize,
      retainPage: false,
    });
  }

  /** Starts page one at a new page size while retaining selections by stable row ID. */
  public async setPageSize(pageSize: RecordPageSize): Promise<void> {
    if (this.#disposed || pageSize === this.#pageSize) return;
    this.#pageSize = pageSize;
    if (this.#state.kind === "initial") return;
    this.#currentCursor = null;
    this.#previousCursors = [];
    await this.load({
      query: this.#query,
      cursor: null,
      previousCursors: [],
      pageSize,
      retainPage: false,
    });
  }

  public async next(): Promise<void> {
    if (this.#disposed || this.#state.kind !== "ready" || this.#state.nextCursor === null) return;
    await this.load({
      query: this.#query,
      cursor: this.#state.nextCursor,
      previousCursors: [...this.#previousCursors, this.#currentCursor],
      pageSize: this.#pageSize,
      retainPage: true,
    });
  }

  public async previous(): Promise<void> {
    if (this.#disposed || this.#previousCursors.length === 0 || this.#state.kind === "loading")
      return;
    const cursor = this.#previousCursors[this.#previousCursors.length - 1] ?? null;
    await this.load({
      query: this.#query,
      cursor,
      previousCursors: this.#previousCursors.slice(0, -1),
      pageSize: this.#pageSize,
      retainPage: true,
    });
  }

  /** Reloads the current cursor without changing its query, page size, or selection. */
  public async refresh(): Promise<void> {
    if (this.#disposed || this.#state.kind === "initial") return;
    await this.load({
      query: this.#query,
      cursor: this.#currentCursor,
      previousCursors: this.#previousCursors,
      pageSize: this.#pageSize,
      retainPage: true,
    });
  }

  /** Repeats exactly the request that most recently failed. */
  public async retry(): Promise<void> {
    if (this.#disposed || this.#failedRequest === undefined) return;
    await this.load(this.#failedRequest);
  }

  /** Restores the definition's starting query without sending a request. */
  public reset(): void {
    if (this.#disposed) return;
    this.#requestNumber += 1;
    this.#query = this.definition.cleanup(this.definition.initialQuery);
    this.#currentCursor = null;
    this.#previousCursors = [];
    this.#failedRequest = undefined;
    this.clearSelection();
    this.setState({ kind: "initial", rows: [], filterCounts: undefined, nextCursor: null });
  }

  /** Makes all current and future asynchronous responses inert. */
  public dispose(): void {
    this.#disposed = true;
    this.#requestNumber += 1;
    this.#failedRequest = undefined;
  }

  /** Selects or deselects a stable row ID, respecting this search's optional maximum. */
  public select(id: string, selected = true): boolean {
    if (this.#disposed) return false;
    if (!selected) {
      this.#selectedIds.delete(id);
      return true;
    }
    if (this.#selectedIds.has(id)) return true;
    const maximum = this.definition.selection?.maximum ?? 0;
    if (this.#selectedIds.size >= maximum) return false;
    this.#selectedIds.add(id);
    return true;
  }

  /** Selects every loaded row that fits in the configured selection maximum. */
  public selectLoaded(): number {
    if (this.#disposed) return 0;
    for (const row of this.#state.rows) this.select(this.definition.rowId(row));
    return this.#selectedIds.size;
  }

  public clearSelection(): void {
    this.#selectedIds.clear();
  }

  /** Removes known stable IDs without disturbing selections retained on another result page. */
  public deselectIds(ids: ReadonlyArray<string>): void {
    if (this.#disposed) return;
    for (const id of ids) this.#selectedIds.delete(id);
  }

  private setState(state: SearchState<Row, FilterCounts>): void {
    if (this.#disposed) return;
    this.#state = state;
    this.publish(state);
  }

  private async load(request: SearchRequest<Query>): Promise<void> {
    const requestNumber = ++this.#requestNumber;
    this.#failedRequest = undefined;
    const previous = this.#state;
    const retainedRows = request.retainPage ? previous.rows : [];
    const retainedFilterCounts = request.retainPage ? previous.filterCounts : undefined;
    const retainedNextCursor = request.retainPage ? previous.nextCursor : null;
    this.setState({
      kind: "loading",
      rows: retainedRows,
      filterCounts: retainedFilterCounts,
      nextCursor: retainedNextCursor,
    });
    let page: SearchPage<Row, FilterCounts>;
    try {
      page = await this.definition.fetchPage(request.query, request.cursor, request.pageSize);
    } catch (error: unknown) {
      if (this.#disposed || requestNumber !== this.#requestNumber) return;
      this.#failedRequest = request;
      this.setState({
        kind: "error",
        rows: retainedRows,
        filterCounts: retainedFilterCounts,
        nextCursor: retainedNextCursor,
        error,
      });
      return;
    }
    if (this.#disposed || requestNumber !== this.#requestNumber) return;
    this.#query = request.query;
    this.#pageSize = request.pageSize;
    this.#currentCursor = request.cursor;
    this.#previousCursors = request.previousCursors;
    this.setState(
      page.items.length === 0
        ? { kind: "empty", rows: [], filterCounts: page.filterCounts, nextCursor: null }
        : {
            kind: "ready",
            rows: page.items,
            filterCounts: page.filterCounts,
            nextCursor: page.nextCursor,
          },
    );
  }
}
