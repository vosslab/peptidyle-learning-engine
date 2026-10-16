import { Show, type Accessor, type JSX } from "solid-js";

import type { RecordCollectionEmptyState } from "../../components/record_list/record_collection_state";
import {
  RecordList,
  type RecordListSelection,
  type RecordListState,
} from "../../components/record_list/record_list";
import type { RecordField, RecordListQuery } from "../../components/record_list/record_fields";
import { RecordPageControls } from "../../components/record_list/record_page_controls";
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
  readonly selection?: () => RecordListSelection<Row> | undefined;
  readonly initialEmptyState?: RecordCollectionEmptyState;
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  readonly query: RecordListQuery<SortKey, FilterKey>;
  /** Caller-owned row content under the shared RecordList identity. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
};

function listState<Query, Row, FilterCounts>(
  state: SearchStateController<Query, Row, FilterCounts>,
): RecordListState {
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

/** Shared current-page records and cursor navigation. */
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
  const hasNext = (): boolean => {
    const current = props.state.state();
    return (
      rows().length > 0 &&
      current.kind !== "initial" &&
      current.kind !== "empty" &&
      current.nextCursor !== null
    );
  };
  const common = {
    get rows(): ReadonlyArray<Row> {
      return rows();
    },
    content: props.state.definition.content,
    recordId: props.state.definition.rowId,
    renderBody: props.renderBody,
    get selection(): RecordListSelection<Row> | undefined {
      return props.selection?.();
    },
    get state(): RecordListState {
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
      <RecordList {...common} fields={props.fields} query={props.query} />
      <Show when={props.state.state().kind !== "initial"}>
        <RecordPageControls
          ariaLabel={`${props.ariaLabel} pages`}
          pageSize={props.state.pageSize()}
          maximumPageSize={props.state.definition.maximumPageSize}
          onPageSizeChange={(size) => void props.state.setPageSize(size)}
          hasPrevious={props.state.hasPrevious()}
          hasNext={hasNext()}
          loading={props.state.state().kind === "loading"}
          disabled={props.disabled?.() || props.state.state().kind === "error"}
          onPrevious={() => void props.state.previous()}
          onNext={() => void props.state.next()}
        />
      </Show>
    </section>
  );
}
