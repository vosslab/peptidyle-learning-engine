import type { Accessor, JSX } from "solid-js";

import type { RecordCollectionEmptyState } from "../data_sheet/record_collection_state";
import type { RecordField, RecordListQuery } from "../data_sheet/record_fields";
import type { DataSheetSelection } from "../data_sheet/data_sheet_state";
import type { DataSheetMode } from "../data_sheet/data_sheet_state";
import type { RecordTransferBinding } from "../data_sheet/record_interactions";
import type { RecordReorderBinding } from "../data_sheet/record_reorder";
import { SearchResults } from "./search_results";
import type { SearchStateController } from "./search_state";
import "./search.css";

export type SearchBlockCollection<Row, SortKey extends string, FilterKey extends string> = {
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly initialEmptyState?: RecordCollectionEmptyState;
  readonly initialMode?: DataSheetMode;
  readonly reorder?: RecordReorderBinding;
  readonly transfer?: RecordTransferBinding<Row>;
  /** The same fields describe empty and populated results in every presentation. */
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  /** The query owner applies collection-wide operations before server paging. */
  readonly query: RecordListQuery<SortKey, FilterKey>;
  /** Caller-owned row content within the shared DataSheetBlock. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: () => DataSheetSelection<Row> | undefined;
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

/** Search controls and one required DataSheetBlock inside a page or embedded picker. */
export function SearchBlock<
  Query,
  Row,
  FilterCounts,
  SortKey extends string = string,
  FilterKey extends string = string,
>(props: SearchBlockProps<Query, Row, FilterCounts, SortKey, FilterKey>): JSX.Element {
  return (
    <div class="search-block" classList={{ "search-block--beside": props.layout === "beside" }}>
      <div class="search-block__layout">
        <div class="search-block__controls">{props.controls}</div>
        <div class="search-block__collection">
          <SearchResults
            state={props.state}
            ariaLabel={props.collection.ariaLabel}
            emptyState={props.collection.emptyState}
            initialEmptyState={props.collection.initialEmptyState}
            initialMode={props.collection.initialMode}
            reorder={props.collection.reorder}
            transfer={props.collection.transfer}
            fields={props.collection.fields}
            query={props.collection.query}
            renderBody={props.collection.renderBody}
            selection={props.collection.selection}
            disabled={props.collection.disabled}
          />
        </div>
      </div>
    </div>
  );
}
