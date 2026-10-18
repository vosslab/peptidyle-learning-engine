import { type Accessor, type JSX } from "solid-js";

import type {
  RecordCollectionEmptyState,
  RecordCollectionState,
} from "../data_sheet/record_collection_state";
import { DataSheetBlock } from "../data_sheet/data_sheet_block";
import { DataSheetControls } from "../data_sheet/data_sheet_controls";
import type { RecordField, RecordListQuery } from "../data_sheet/record_fields";
import type { RecordPageControlsProps } from "../data_sheet/record_page_controls";
import type { DataSheetSelection } from "../data_sheet/data_sheet_state";
import {
  RecordInteractionStateProvider,
  type RecordTransferBinding,
} from "../data_sheet/record_interactions";
import type { RecordReorderBinding } from "../data_sheet/record_reorder";
import type { DataSheetMode } from "../data_sheet/data_sheet_state";
import type { SearchStateController } from "./search_state";
import "./search.css";

export type SearchResultsProps<
  Query,
  Row,
  FilterCounts,
  SortKey extends string = string,
  FilterKey extends string = string,
> = {
  readonly state: SearchStateController<Query, Row, FilterCounts>;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  /** Content work, such as a bulk editor, can temporarily hold navigation still. */
  readonly disabled?: () => boolean;
  readonly selection?: () => DataSheetSelection<Row> | undefined;
  readonly initialEmptyState?: RecordCollectionEmptyState;
  readonly initialMode?: DataSheetMode;
  readonly reorder?: RecordReorderBinding;
  readonly transfer?: RecordTransferBinding<Row>;
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  readonly query: RecordListQuery<SortKey, FilterKey>;
  /** Caller-owned row content within the shared DataSheetBlock. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
};

function listState<Query, Row, FilterCounts>(
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

/** Shared current-page records and numbered navigation composed through DataSheetBlock. */
export function SearchResults<
  Query,
  Row,
  FilterCounts,
  SortKey extends string = string,
  FilterKey extends string = string,
>(props: SearchResultsProps<Query, Row, FilterCounts, SortKey, FilterKey>): JSX.Element {
  const rows = (): ReadonlyArray<Row> => {
    const current = props.state.state();
    return current.kind === "empty" ? [] : current.rows;
  };
  const common = {
    get rows(): ReadonlyArray<Row> {
      return rows();
    },
    content: props.state.definition.content,
    recordId: props.state.definition.rowId,
    renderBody: props.renderBody,
    get selection(): DataSheetSelection<Row> | undefined {
      return props.selection?.();
    },
    get state(): RecordCollectionState {
      return listState(props.state);
    },
    ariaLabel: props.ariaLabel,
    get emptyState(): RecordCollectionEmptyState {
      return props.state.state().kind === "initial"
        ? (props.initialEmptyState ?? props.emptyState)
        : props.emptyState;
    },
  };
  return (
    <section class="shared-search-results" aria-label={props.ariaLabel}>
      <RecordInteractionStateProvider state={props.state.state()}>
        <DataSheetBlock
          {...common}
          reorder={props.reorder}
          transfer={props.transfer}
          disabled={props.disabled}
          initialMode={props.initialMode}
          fields={props.fields}
          query={props.query}
          controls={(presentation, table) => (
            <DataSheetControls
              presentation={presentation}
              table={table}
              fields={props.fields}
              query={props.query}
              selection={props.selection?.()}
              loadedCount={rows().length}
              pagination={
                props.state.state().kind === "initial"
                  ? undefined
                  : ({
                      ariaLabel: `${props.ariaLabel} pages`,
                      pageSize: props.state.pageSize(),
                      maximumPageSize: props.state.definition.maximumPageSize,
                      onPageSizeChange: (size) => void props.state.setPageSize(size),
                      pageNumber: props.state.pageNumber(),
                      totalCount: props.state.totalCount(),
                      hasPrevious: props.state.hasPrevious(),
                      hasNext: props.state.pageNumber() < props.state.totalPages(),
                      loading: props.state.state().kind === "loading",
                      disabled: props.disabled?.() || props.state.state().kind === "error",
                      onPrevious: () => void props.state.previous(),
                      onNext: () => void props.state.next(),
                      onGoToPage: (pageNumber) => void props.state.goToPage(pageNumber),
                    } satisfies RecordPageControlsProps)
              }
            />
          )}
        />
      </RecordInteractionStateProvider>
    </section>
  );
}
