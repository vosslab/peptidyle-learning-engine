import { For, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";

export type RecordTableColumn<Row> = {
  readonly id: string;
  readonly header: JSX.Element;
  readonly cell: (row: Row) => JSX.Element;
  readonly width?: string;
  readonly align?: "start" | "center" | "end";
};

export type RecordTableRowHeader<Row> = {
  readonly id: string;
  readonly header: JSX.Element;
  readonly content: (row: Row) => JSX.Element;
  readonly width?: string;
};

export type RecordTableProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly columns: ReadonlyArray<RecordTableColumn<Row>>;
  readonly rowId: (row: Row) => string;
  readonly rowHeader: RecordTableRowHeader<Row>;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
};

/** Semantic records with named columns and one row header per record. */
export function RecordTable<Row>(props: RecordTableProps<Row>): JSX.Element {
  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.rows.length === 0}
      emptyState={props.emptyState}
    >
      <div class="record-table__scroll">
        <table class="record-table" aria-label={props.ariaLabel}>
          <thead>
            <tr>
              <th
                scope="col"
                data-record-column-id={props.rowHeader.id}
                style={{ width: props.rowHeader.width }}
              >
                {props.rowHeader.header}
              </th>
              <For each={props.columns}>
                {(column) => (
                  <th
                    scope="col"
                    data-record-column-id={column.id}
                    style={{ width: column.width, "text-align": column.align ?? "start" }}
                  >
                    {column.header}
                  </th>
                )}
              </For>
            </tr>
          </thead>
          <tbody>
            <For each={props.rows}>
              {(row) => (
                <tr data-record-id={props.rowId(row)}>
                  <th scope="row" data-record-column-id={props.rowHeader.id}>
                    {props.rowHeader.content(row)}
                  </th>
                  <For each={props.columns}>
                    {(column) => (
                      <td
                        data-record-column-id={column.id}
                        style={{ "text-align": column.align ?? "start" }}
                      >
                        {column.cell(row)}
                      </td>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </RecordCollectionStateView>
  );
}
