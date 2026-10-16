import type { Accessor, JSX } from "solid-js";

import type { RecordCollectionEmptyState } from "../../components/record_list/record_collection_state";
import type { RecordField, RecordListQuery } from "../../components/record_list/record_fields";
import type { RecordListSelection } from "../../components/record_list/record_list";
import { SearchResults } from "./search_results";
import type { SearchStateController } from "./search_state";
import "./search.css";

export type SearchBlockCollection<Row, SortKey extends string, FilterKey extends string> = {
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly initialEmptyState?: RecordCollectionEmptyState;
  /** The same fields describe empty and populated results in every presentation. */
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  /** The query owner applies collection-wide operations before server paging. */
  readonly query: RecordListQuery<SortKey, FilterKey>;
  /** Caller-owned row content under the shared RecordList identity. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: () => RecordListSelection<Row> | undefined;
  readonly disabled?: () => boolean;
};

export type SearchBlockProps<
  Query,
  Row,
  FilterCounts,
  SortKey extends string,
  FilterKey extends string,
> = {
  readonly layout: "above" | "beside";
  readonly controls: JSX.Element;
  readonly state: SearchStateController<Query, Row, FilterCounts>;
  readonly collection: SearchBlockCollection<Row, SortKey, FilterKey>;
};

/** Search controls and one required RecordList inside a page or embedded picker. */
export function SearchBlock<
  Query,
  Row,
  FilterCounts,
  SortKey extends string = string,
  FilterKey extends string = string,
>(props: SearchBlockProps<Query, Row, FilterCounts, SortKey, FilterKey>): JSX.Element {
  return (
    <div class="search-block" classList={{ "search-block--beside": props.layout === "beside" }}>
      <div class="search-block__controls">{props.controls}</div>
      <div class="search-block__collection">
        <SearchResults
          state={props.state}
          ariaLabel={props.collection.ariaLabel}
          emptyState={props.collection.emptyState}
          initialEmptyState={props.collection.initialEmptyState}
          fields={props.collection.fields}
          query={props.collection.query}
          renderBody={props.collection.renderBody}
          selection={props.collection.selection}
          disabled={props.collection.disabled}
        />
      </div>
    </div>
  );
}
