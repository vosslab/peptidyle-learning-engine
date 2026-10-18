import type { Accessor, JSX } from "solid-js";

import type {
  RecordCollectionEmptyState,
  RecordCollectionState,
} from "../data_sheet/data_sheet_collection_state";
import { DataSheetBlock } from "../data_sheet/data_sheet_block";
import type { RecordField, DataSheetQuery } from "../data_sheet/data_sheet_fields";
import type { RecordPageControlsProps } from "../data_sheet/data_sheet_page_controls";
import type { DataSheetSelection } from "../data_sheet/data_sheet_state";
import type { DataSheetMode } from "../data_sheet/data_sheet_state";
import {
  RecordInteractionStateProvider,
  type RecordTransferBinding,
} from "../data_sheet/data_sheet_interactions";
import type { RecordReorderBinding } from "../data_sheet/record_reorder";
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
  readonly query: DataSheetQuery<SortKey, FilterKey>;
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

function collectionState<Query, Row, FilterCounts>(
  state: SearchStateController<Query, Row, FilterCounts>,
): RecordCollectionState {
  const current = state.state();
  if (current.kind === "loading") return { kind: "loading", label: "Loading results..." };
  if (current.kind === "error") {
    return {
      kind: "error",
      title: "Search could not load",
      message: "Your search choices are still here. Check the connection and try again.",
      retry: () => void state.retry(),
      retryLabel: "Try again",
    };
  }
  return { kind: "ready" };
}

/** Search controls and one required DataSheetBlock inside a page or embedded picker. */
export function SearchBlock<
  Query,
  Row,
  FilterCounts,
  SortKey extends string = string,
  FilterKey extends string = string,
>(props: SearchBlockProps<Query, Row, FilterCounts, SortKey, FilterKey>): JSX.Element {
  const rows = (): ReadonlyArray<Row> => {
    const current = props.state.state();
    return current.kind === "empty" ? [] : current.rows;
  };
  const collection = {
    get rows(): ReadonlyArray<Row> {
      return rows();
    },
    content: props.state.definition.content,
    recordId: props.state.definition.rowId,
    renderBody: props.collection.renderBody,
    get selection(): DataSheetSelection<Row> | undefined {
      return props.collection.selection?.();
    },
    get state(): RecordCollectionState {
      return collectionState(props.state);
    },
    ariaLabel: props.collection.ariaLabel,
    get emptyState(): RecordCollectionEmptyState {
      return props.state.state().kind === "initial"
        ? (props.collection.initialEmptyState ?? props.collection.emptyState)
        : props.collection.emptyState;
    },
    get pagination(): RecordPageControlsProps | undefined {
      if (props.state.state().kind === "initial") return undefined;
      return {
        ariaLabel: `${props.collection.ariaLabel} pages`,
        pageSize: props.state.pageSize(),
        maximumPageSize: props.state.definition.maximumPageSize,
        onPageSizeChange: (size) => void props.state.setPageSize(size),
        pageNumber: props.state.pageNumber(),
        totalCount: props.state.totalCount(),
        hasPrevious: props.state.hasPrevious(),
        hasNext: props.state.pageNumber() < props.state.totalPages(),
        loading: props.state.state().kind === "loading",
        disabled: props.collection.disabled?.() || props.state.state().kind === "error",
        onPrevious: () => void props.state.previous(),
        onNext: () => void props.state.next(),
        onGoToPage: (pageNumber) => void props.state.goToPage(pageNumber),
      };
    },
  };
  return (
    <div class="search-block" classList={{ "search-block--beside": props.layout === "beside" }}>
      <div class="search-block__layout">
        <div class="search-block__controls">{props.controls}</div>
        <div class="search-block__collection">
          <section class="shared-search-results" aria-label={props.collection.ariaLabel}>
            <RecordInteractionStateProvider state={props.state.state()}>
              <DataSheetBlock
                {...collection}
                reorder={props.collection.reorder}
                transfer={props.collection.transfer}
                disabled={props.collection.disabled}
                initialMode={props.collection.initialMode}
                fields={props.collection.fields}
                query={props.collection.query}
              />
            </RecordInteractionStateProvider>
          </section>
        </div>
      </div>
    </div>
  );
}
