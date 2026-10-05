import { Show, type JSX } from "solid-js";

import {
  RecordList,
  type RecordListSelection,
  type RecordListState,
} from "../../components/record_list/record_list";
import { RecordPageControls } from "../../components/record_list/record_page_controls";
import type { SearchStateController } from "./search_state";
import "./search.css";

export type SearchResultsProps<Query, Row, FilterCounts, Sort extends string = never> = {
  readonly state: SearchStateController<Query, Row, FilterCounts, Sort>;
  readonly ariaLabel: string;
  readonly emptyState: { readonly title: string; readonly message: string };
  /** Content work, such as a bulk editor, can temporarily hold navigation still. */
  readonly disabled?: () => boolean;
  readonly selection?: () => RecordListSelection<Row> | undefined;
  /** A caller can withhold rows until its discovery policy has enough filters. */
  readonly rows?: () => ReadonlyArray<Row>;
};

function listState<Query, Row, FilterCounts, Sort extends string>(
  state: SearchStateController<Query, Row, FilterCounts, Sort>,
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
export function SearchResults<Query, Row, FilterCounts, Sort extends string = never>(
  props: SearchResultsProps<Query, Row, FilterCounts, Sort>,
): JSX.Element {
  const rows = (): ReadonlyArray<Row> => {
    if (props.rows !== undefined) return props.rows();
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
  return (
    <Show when={props.state.state().kind !== "initial"}>
      <section class="shared-search-results" aria-label={props.ariaLabel}>
        <RecordList
          rows={rows()}
          content={props.state.definition.content}
          recordId={props.state.definition.rowId}
          presentation={props.state.display()}
          selection={props.selection?.()}
          state={listState(props.state)}
          ariaLabel={props.ariaLabel}
          emptyState={props.emptyState}
        />
        <RecordPageControls
          ariaLabel={`${props.ariaLabel} pages`}
          pageSize={props.state.pageSize()}
          onPageSizeChange={(size) => void props.state.setPageSize(size)}
          hasPrevious={props.state.hasPrevious()}
          hasNext={hasNext()}
          loading={props.state.state().kind === "loading"}
          disabled={props.disabled?.() || props.state.state().kind === "error"}
          onPrevious={() => void props.state.previous()}
          onNext={() => void props.state.next()}
        />
      </section>
    </Show>
  );
}
