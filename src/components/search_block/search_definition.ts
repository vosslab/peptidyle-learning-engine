import type { RecordContent } from "../data_sheet/record_content";
import type { RecordPageSize } from "../data_sheet/data_sheet_page_sizes";

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
 * Query cleanup and response decoding belong to the definition. Query owners must order and
 * count the complete authorized result set before slicing its requested page.
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
