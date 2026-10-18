import { For, Show, createMemo, createUniqueId, mapArray, type Accessor, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import { recordIdentityField, type RecordField, type RecordListQuery } from "./record_fields";
import { DataSheetTable } from "./data_sheet_table";
import { createDataSheetTableState, type DataSheetTableState } from "./data_sheet_table_state";
import {
  createDataSheetPresentation,
  type DataSheetPresentation,
  type DataSheetSelection,
} from "./data_sheet_state";
import { RecordItem } from "./record_item";
import { DataSheetInteractions, type RecordTransferBinding } from "./record_interactions";
import type { RecordReorderBinding } from "./record_reorder";
import type { RecordContent } from "./record_content";
import type { DataSheetMode } from "./data_sheet_state";
import "./data_sheet.css";

export type DataSheetBlockProps<
  Row,
  SortKey extends string = string,
  FilterKey extends string = string,
> = {
  readonly rows: ReadonlyArray<Row>;
  readonly recordId: (row: Row) => string;
  readonly content: (row: Row) => RecordContent;
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: DataSheetSelection<Row>;
  readonly reorder?: RecordReorderBinding;
  readonly transfer?: RecordTransferBinding<Row>;
  readonly disabled?: () => boolean;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly initialMode?: DataSheetMode;
  /** Rendered beside, rather than inside, the DataSheet record/table region. */
  readonly controls?: (
    presentation: DataSheetPresentation<FilterKey>,
    table: DataSheetTableState<Row, SortKey, FilterKey>,
  ) => JSX.Element;
} & (
  | { readonly fields?: never; readonly query?: never }
  | {
      readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
      readonly query: RecordListQuery<SortKey, FilterKey>;
    }
);

/** The shared record table and presentation owner. Query-wide work remains in the query layer. */
export function DataSheetBlock<
  Row,
  SortKey extends string = string,
  FilterKey extends string = string,
>(props: DataSheetBlockProps<Row, SortKey, FilterKey>): JSX.Element {
  const fields = (): ReadonlyArray<RecordField<Row, SortKey, FilterKey>> => props.fields ?? [];
  const identityField = createMemo(() => recordIdentityField(fields()));
  const rowsById = createMemo(() => new Map(props.rows.map((row) => [props.recordId(row), row])));
  const recordIds = createMemo(() => props.rows.map((row) => props.recordId(row)));
  const bodyModels = mapArray(recordIds, (recordId) => ({
    recordId,
    body: props.renderBody?.(() => rowsById().get(recordId)!),
  }));
  const bodiesById = createMemo(
    () => new Map(bodyModels().map((model) => [model.recordId, model.body])),
  );
  const selectionName = `data-sheet-selection-${createUniqueId()}`;
  const recordContent = (record: Row): RecordContent => {
    const content = props.content(record);
    const identity = identityField();
    return identity === undefined ? content : { ...content, title: identity.value(record) };
  };
  const hasRecordActions = createMemo(() =>
    props.rows.some((record) => props.content(record).actions.length > 0),
  );
  const presentation = createDataSheetPresentation<FilterKey>(props.initialMode);
  const table = createDataSheetTableState({
    mode: presentation.mode,
    fields,
    identityField,
    hasRecordActions,
  });
  const row = (recordId: string): Row => rowsById().get(recordId)!;
  const semanticPresentation = (): "cards" | "visual_boxes" =>
    presentation.mode() === "cards" ? "cards" : "visual_boxes";
  return (
    <>
      <Show when={props.controls}>{(controls) => controls()(presentation, table)}</Show>
      <DataSheetInteractions
        reorder={props.reorder}
        transfer={props.transfer}
        row={(id) => rowsById().get(id)}
        recordIds={recordIds()}
        ready={props.state.kind === "ready" && !props.disabled?.()}
        ariaLabel={props.ariaLabel}
      >
        <Show
          when={presentation.mode() === "compact" || presentation.mode() === "list"}
          fallback={
            <RecordCollectionStateView
              state={props.state}
              isEmpty={props.rows.length === 0}
              emptyState={props.emptyState}
            >
              <div
                class="record-list record-list--semantic"
                classList={{
                  "record-list--cards": presentation.mode() === "cards",
                  "record-list--visual-boxes": presentation.mode() === "visual_boxes",
                }}
                role="list"
                aria-label={props.ariaLabel}
              >
                <For each={recordIds()}>
                  {(recordId) => (
                    <RecordItem
                      recordId={recordId}
                      row={() => row(recordId)}
                      content={recordContent}
                      semanticPresentation={semanticPresentation()}
                      renderBody={
                        props.renderBody === undefined
                          ? undefined
                          : (): JSX.Element => bodiesById().get(recordId)
                      }
                      selection={props.selection}
                      selectionName={selectionName}
                    />
                  )}
                </For>
              </div>
            </RecordCollectionStateView>
          }
        >
          <DataSheetTable
            recordIds={recordIds()}
            row={row}
            content={recordContent}
            renderBody={
              props.renderBody === undefined
                ? undefined
                : (record): JSX.Element => bodiesById().get(props.recordId(record()))
            }
            selection={props.selection}
            selectionName={selectionName}
            identityField={identityField()}
            hasRecordActions={hasRecordActions()}
            query={props.query}
            presentation={presentation}
            table={table}
            state={props.state}
            ariaLabel={props.ariaLabel}
            emptyState={props.emptyState}
          />
        </Show>
      </DataSheetInteractions>
    </>
  );
}
