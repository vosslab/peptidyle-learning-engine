import { For, Show, createMemo, type Accessor, type JSX } from "solid-js";

import { RecordInteractionControls, useRecordInteractions } from "./data_sheet_interactions";

import type { RecordFieldValue } from "./data_sheet_fields";
import { RecordActionControl } from "./record_action_control";
import {
  normalizeRecordActions,
  RecordSemanticContent,
  RecordIdentityContent,
  RecordPreview,
  type RecordContent,
} from "./record_content";
import type { DataSheetSelection } from "./data_sheet_state";
import type { DataSheetMode } from "./data_sheet_state";

function selectionControl<Row>(props: {
  readonly recordId: string;
  readonly row: Accessor<Row>;
  readonly content: Accessor<RecordContent>;
  readonly selection?: DataSheetSelection<Row>;
  readonly selectionName: string;
  readonly compact: boolean;
}): JSX.Element | undefined {
  if (props.selection === undefined) return undefined;
  const selection = props.selection;
  return (
    <label classList={{ "record-list__compact-selection": props.compact }}>
      <input
        type={selection.kind}
        name={selection.kind === "radio" ? props.selectionName : undefined}
        value={props.recordId}
        checked={selection.selectedIds().has(props.recordId)}
        disabled={selection.disabled?.(props.row())}
        onChange={(event) => {
          const input = event.currentTarget;
          try {
            selection.onChange(props.row(), input.checked);
          } finally {
            if (selection.kind === "checkbox") {
              input.checked = selection.selectedIds().has(props.recordId);
            }
          }
        }}
      />
      <span class="sr-only">Select {props.content().title}</span>
    </label>
  );
}

export function fieldText(value: RecordFieldValue): string {
  if (value === null) return "";
  return Array.isArray(value) ? value.map(String).join(", ") : String(value);
}

/** Renders a record in table or semantic modes; DataSheetBlock owns selection and identity. */
export function RecordItem<Row>(props: {
  readonly recordId: string;
  readonly row: Accessor<Row>;
  readonly content: (row: Row) => RecordContent;
  readonly presentation?: "compact" | "list";
  readonly semanticPresentation?: Extract<DataSheetMode, "cards" | "visual_boxes">;
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: DataSheetSelection<Row>;
  readonly selectionName: string;
}): JSX.Element {
  const content = createMemo(() => props.content(props.row()));
  const interactions = useRecordInteractions();
  return (
    <Show
      when={props.presentation !== undefined}
      fallback={
        <article
          class="record-list__row record-list__row--semantic"
          role="listitem"
          data-record-id={props.recordId}
          classList={{
            "record-list__row--selectable": props.selection !== undefined,
            "record-list__card": props.semanticPresentation === "cards",
            "record-list__visual-box": props.semanticPresentation === "visual_boxes",
            "record-list__row--transfer-target": interactions?.canDrop(props.recordId),
          }}
          onDragOver={(event) => interactions?.allowDrop(event, props.recordId)}
          onDrop={(event) => interactions?.completeDrop(event, props.recordId)}
        >
          <Show when={props.selection}>
            <div class="record-list__selection">
              {selectionControl({ ...props, content, compact: false })}
            </div>
          </Show>
          <RecordInteractionControls recordId={props.recordId} recordLabel={content().title} />
          <RecordSemanticContent
            recordId={props.recordId}
            content={content}
            renderBody={
              props.renderBody === undefined
                ? undefined
                : (): JSX.Element => props.renderBody!(props.row)
            }
          />
        </article>
      }
    >
      <Show
        when={props.presentation === "compact"}
        fallback={
          <div class="record-list__list-identity">
            <RecordInteractionControls recordId={props.recordId} recordLabel={content().title} />
            {selectionControl({ ...props, content, compact: false })}
            <div>
              <RecordIdentityContent content={content} />
              <RecordPreview content={content} />
              <Show when={props.renderBody}>
                {(body) => <div class="record-list__body">{body()(props.row)}</div>}
              </Show>
            </div>
          </div>
        }
      >
        <CompactRecordIdentity {...props} />
      </Show>
    </Show>
  );
}

/** Compact row-header identity shared with DataSheetBlock's table mechanics. */
export function CompactRecordIdentity<Row>(props: {
  readonly recordId: string;
  readonly row: Accessor<Row>;
  readonly content: (row: Row) => RecordContent;
  readonly selection?: DataSheetSelection<Row>;
  readonly selectionName: string;
}): JSX.Element {
  const content = createMemo(() => props.content(props.row()));
  return (
    <div class="record-list__compact-identity">
      {selectionControl({ ...props, content, compact: true })}
      <span class="record-list__compact-title" title={content().title}>
        {content().title}
      </span>
      <RecordInteractionControls recordId={props.recordId} recordLabel={content().title} />
    </div>
  );
}

export function CompactRecordActions(props: {
  readonly recordId: string;
  readonly content: Accessor<RecordContent>;
}): JSX.Element {
  const actions = createMemo(() => normalizeRecordActions(props.recordId, props.content().actions));
  return (
    <div class="record-list__compact-actions">
      <For each={actions().ids}>
        {(actionId) => <RecordActionControl action={() => actions().byId.get(actionId)!} />}
      </For>
    </div>
  );
}
