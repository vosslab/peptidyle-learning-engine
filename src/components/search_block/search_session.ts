import type { RecordContent } from "../data_sheet/record_content";
import { RECORD_PAGE_SIZES, type RecordPageSize } from "../data_sheet/record_page_sizes";

/** A numbered request for one complete, ordered result set. */
export type SearchPageRequest = {
  /** One-based page number requested by the collection controls. */
  readonly pageNumber: number;
  /** A validated shared record page size. */
  readonly pageSize: RecordPageSize;
};

/** One resolved numbered page for a content-specific search definition. */
export type SearchPage<Row, FilterCounts> = {
  readonly items: ReadonlyArray<Row>;
  /** One-based page number resolved by the query owner after clamping. */
  readonly pageNumber: number;
  readonly pageSize: RecordPageSize;
  /** Complete result count after the query owner has applied scope, filters, and ordering. */
  readonly totalCount: number;
  /** Counts describe the exact query that produced this page. */
  readonly filterCounts: FilterCounts;
};

/** A removable filter chip supplied by a content-specific search definition. */
export type SearchAppliedFilter<Query> = {
  readonly id: string;
  readonly label: string;
  readonly clear: (query: Query) => Query;
};

/**
 * The content-owned boundary around reusable numbered search machinery.
 *
 * Query cleanup and response decoding belong to the definition. The session owns one resolved
 * numbered page, request ordering, and row-ID selection. Query owners must order and count the
 * complete authorized result set before slicing its requested page.
 */
export type SearchDefinition<Query, Row, FilterCounts> = {
  readonly initialQuery: Query;
  /** Largest page size supported by this query owner; defaults to the shared maximum. */
  readonly maximumPageSize?: RecordPageSize;
  readonly cleanup: (query: Query) => Query;
  /** Whether this cleaned query may request result rows. */
  readonly isEligible: (query: Query) => boolean;
  readonly getText: (query: Query) => string;
  readonly setText: (query: Query, text: string) => Query;
  readonly fetchPage: (
    query: Query,
    page: SearchPageRequest,
  ) => Promise<SearchPage<Row, FilterCounts>>;
  readonly rowId: (row: Row) => string;
  readonly content: (row: Row) => RecordContent;
  readonly appliedFilters?: (query: Query) => ReadonlyArray<SearchAppliedFilter<Query>>;
  readonly selection?: { readonly maximum: number };
};

type NumberedState<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly pageNumber: number;
  readonly pageSize: RecordPageSize;
  readonly totalCount: number;
};

export type SearchState<Row, FilterCounts> =
  | (NumberedState<Row> & { readonly kind: "initial"; readonly filterCounts: undefined })
  | (NumberedState<Row> & {
      readonly kind: "loading";
      readonly filterCounts: FilterCounts | undefined;
    })
  | (NumberedState<Row> & { readonly kind: "ready"; readonly filterCounts: FilterCounts })
  | (NumberedState<Row> & { readonly kind: "empty"; readonly filterCounts: FilterCounts })
  | (NumberedState<Row> & {
      readonly kind: "error";
      readonly filterCounts: FilterCounts | undefined;
      readonly error: unknown;
    });

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

/**
 * Plain TypeScript state for one shared numbered search. It does not depend on a UI runtime.
 *
 * A later request always wins. Criteria changes clear selection immediately; sort, page changes,
 * page-size changes, retries, and refresh preserve stable IDs.
 */
export class SearchSession<Query, Row, FilterCounts> {
  #requestNumber = 0;
  #disposed = false;
  #query: Query;
  #pageSize: RecordPageSize = DEFAULT_PAGE_SIZE;
  #pageNumber = 1;
  #failedRequest: SearchRequest<Query> | undefined;
  #selectedIds = new Set<string>();
  #state: SearchState<Row, FilterCounts> = initialState<Row>(DEFAULT_PAGE_SIZE);

  public constructor(
    private readonly definition: SearchDefinition<Query, Row, FilterCounts>,
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

  public get pageNumber(): number {
    return this.#pageNumber;
  }

  public get totalCount(): number {
    return this.#state.totalCount;
  }

  public get totalPages(): number {
    return pageCount(this.totalCount, this.#pageSize);
  }

  public get selectedIds(): ReadonlySet<string> {
    return this.#selectedIds;
  }

  public get hasPrevious(): boolean {
    return this.#pageNumber > 1;
  }

  public get hasNext(): boolean {
    return this.#pageNumber < this.totalPages;
  }

  public get loading(): boolean {
    return this.#state.kind === "loading";
  }

  /** Starts a changed query on page one and clears selection. */
  public async apply(query: Query): Promise<void> {
    await this.startAtFirstPage(query, true);
  }

  /** Applies a cleaned ordering as page one while retaining selection by stable row ID. */
  public async sort(query: Query): Promise<void> {
    await this.startAtFirstPage(query, false);
  }

  private async startAtFirstPage(query: Query, clearSelection: boolean): Promise<void> {
    if (this.#disposed) return;
    this.#query = this.definition.cleanup(query);
    this.#pageNumber = 1;
    if (clearSelection) this.clearSelection();
    if (!this.definition.isEligible(this.#query)) {
      this.#requestNumber += 1;
      this.#failedRequest = undefined;
      this.setState(initialState<Row>(this.#pageSize));
      return;
    }
    await this.load({
      query: this.#query,
      page: { pageNumber: 1, pageSize: this.#pageSize },
      retainPage: false,
    });
  }

  /**
   * Changes page size while retaining selection and keeping the old page's first record visible.
   * The query owner resolves an out-of-range requested page after counting the complete result set.
   */
  public async setPageSize(pageSize: RecordPageSize): Promise<void> {
    if (
      this.#disposed ||
      !RECORD_PAGE_SIZES.includes(pageSize) ||
      pageSize >
        (this.definition.maximumPageSize ?? RECORD_PAGE_SIZES[RECORD_PAGE_SIZES.length - 1]!) ||
      pageSize === this.#pageSize
    )
      return;
    const firstRecordOffset = (this.#pageNumber - 1) * this.#pageSize;
    const requestedPageNumber = Math.floor(firstRecordOffset / pageSize) + 1;
    this.#pageSize = pageSize;
    if (this.#state.kind === "initial") {
      this.setState(initialState<Row>(pageSize));
      return;
    }
    await this.load({
      query: this.#query,
      page: { pageNumber: requestedPageNumber, pageSize },
      retainPage: true,
    });
  }

  /** Loads a direct one-based page request; the response carries the resolved page number. */
  public async goToPage(pageNumber: number): Promise<void> {
    if (this.#disposed || this.#state.kind === "initial" || !isValidPageNumber(pageNumber)) return;
    await this.load({
      query: this.#query,
      page: { pageNumber, pageSize: this.#pageSize },
      retainPage: true,
    });
  }

  public async next(): Promise<void> {
    if (this.#disposed || this.#state.kind !== "ready" || !this.hasNext) return;
    await this.goToPage(this.#pageNumber + 1);
  }

  public async previous(): Promise<void> {
    if (this.#disposed || this.#state.kind === "loading" || !this.hasPrevious) return;
    await this.goToPage(this.#pageNumber - 1);
  }

  /** Reloads the resolved page without changing its query, page size, or selection. */
  public async refresh(): Promise<void> {
    if (this.#disposed || this.#state.kind === "initial") return;
    await this.load({
      query: this.#query,
      page: { pageNumber: this.#pageNumber, pageSize: this.#pageSize },
      retainPage: true,
    });
  }

  /** Repeats exactly the numbered request that most recently failed. */
  public async retry(): Promise<void> {
    if (this.#disposed || this.#failedRequest === undefined) return;
    await this.load(this.#failedRequest);
  }

  /** Restores the definition's starting query without sending a request. */
  public reset(): void {
    if (this.#disposed) return;
    this.#requestNumber += 1;
    this.#query = this.definition.cleanup(this.definition.initialQuery);
    this.#pageNumber = 1;
    this.#failedRequest = undefined;
    this.clearSelection();
    this.setState(initialState<Row>(this.#pageSize));
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

  private validatePage(page: SearchPage<Row, FilterCounts>, requested: SearchPageRequest): void {
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

  private async load(request: SearchRequest<Query>): Promise<void> {
    const requestNumber = ++this.#requestNumber;
    this.#failedRequest = undefined;
    const previous = this.#state;
    const retainedRows = request.retainPage ? previous.rows : [];
    const retainedFilterCounts = request.retainPage ? previous.filterCounts : undefined;
    const retainedTotalCount = request.retainPage ? previous.totalCount : 0;
    const retainedPageNumber = request.retainPage ? previous.pageNumber : request.page.pageNumber;
    this.setState({
      kind: "loading",
      rows: retainedRows,
      filterCounts: retainedFilterCounts,
      pageNumber: retainedPageNumber,
      pageSize: request.page.pageSize,
      totalCount: retainedTotalCount,
    });
    let page: SearchPage<Row, FilterCounts>;
    try {
      page = await this.definition.fetchPage(request.query, request.page);
      this.validatePage(page, request.page);
    } catch (error: unknown) {
      if (this.#disposed || requestNumber !== this.#requestNumber) return;
      this.#failedRequest = request;
      this.setState({
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
    if (this.#disposed || requestNumber !== this.#requestNumber) return;
    this.#query = request.query;
    this.#pageSize = page.pageSize;
    this.#pageNumber = page.pageNumber;
    this.setState(
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
}
