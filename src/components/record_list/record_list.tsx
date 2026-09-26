import { For, Show, createMemo, createUniqueId, type Accessor, type JSX } from "solid-js";

import { assessmentTypePresentation } from "../../assessment_type_presentation";
import { CopyableQuestionId } from "../copyable_question_id";
import { CourseClassificationSummary } from "../course_classification_summary";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { CourseClassification } from "../../../generated/api/CourseClassification";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import "./record_list.css";

export type RecordListState = RecordCollectionState;
export type RecordListEmptyState = RecordCollectionEmptyState;

export type RecordFact =
  | { readonly kind: "text"; readonly value: string; readonly label?: string }
  | {
      readonly kind: "time";
      readonly value: string;
      readonly dateTime: string;
      readonly label?: string;
    }
  | {
      readonly kind: "link";
      readonly label: string;
      readonly href: string;
      readonly onFollow?: (
        event: MouseEvent & { readonly currentTarget: HTMLAnchorElement },
      ) => void;
    }
  | { readonly kind: "assessmentType"; readonly value: AssessmentType }
  | { readonly kind: "questionId"; readonly questionTitle: string; readonly displayId: string }
  | { readonly kind: "courseClassification"; readonly value: CourseClassification };

export type RecordMedia = {
  readonly src: string;
  readonly alt: string;
};

type RecordActionBase = {
  /** Unique within the containing record. Duplicate IDs are rejected at render time. */
  readonly id: string;
  readonly label: string;
  readonly primary?: boolean;
  readonly title?: string;
};

export type RecordAction =
  | (RecordActionBase & {
      readonly kind: "link";
      readonly href: string;
      readonly onFollow?: (
        event: MouseEvent & { readonly currentTarget: HTMLAnchorElement },
      ) => void;
      readonly ref?: (element: HTMLAnchorElement) => void;
    })
  | (RecordActionBase & {
      readonly kind: "command";
      readonly onClick: (event: MouseEvent & { readonly currentTarget: HTMLButtonElement }) => void;
      readonly disabled?: boolean;
      readonly pressed?: boolean;
      readonly expanded?: boolean;
      readonly controls?: string;
      readonly ref?: (element: HTMLButtonElement) => void;
    });

export type RecordContent = {
  readonly title: string;
  readonly description?: string;
  readonly details: ReadonlyArray<RecordFact>;
  readonly media?: RecordMedia;
  readonly actions: ReadonlyArray<RecordAction>;
};

/** Caller-owned selection state rendered with the shared record treatment. */
export type RecordListSelection<Row> = {
  readonly kind: "checkbox" | "radio";
  readonly selectedIds: Accessor<ReadonlySet<string>>;
  readonly disabled?: (row: Row) => boolean;
  readonly onChange: (row: Row, selected: boolean) => void;
};

export type RecordListProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly recordId: (row: Row) => string;
  readonly content: (row: Row) => RecordContent;
  /** Bounded caller-owned body under the shared identity. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: RecordListSelection<Row>;
  /** Shared List/Gallery presentation for ordinary image records. */
  readonly presentation?: "list" | "gallery";
  readonly state: RecordListState;
  readonly ariaLabel: string;
  readonly emptyState: RecordListEmptyState;
};

function RecordFactView(props: { readonly fact: RecordFact }): JSX.Element {
  const content = (): JSX.Element => {
    switch (props.fact.kind) {
      case "text":
        return <span>{props.fact.value}</span>;
      case "time":
        return <time datetime={props.fact.dateTime}>{props.fact.value}</time>;
      case "link":
        return (
          <a href={props.fact.href} onClick={props.fact.onFollow}>
            {props.fact.label}
          </a>
        );
      case "assessmentType": {
        const type = assessmentTypePresentation(props.fact.value);
        return (
          <span class="record-list__assessment-type" style={{ color: `var(${type.colorToken})` }}>
            <RibbonIcon glyph={type.icon} />
            {type.label}
          </span>
        );
      }
      case "questionId":
        return (
          <CopyableQuestionId
            questionTitle={props.fact.questionTitle}
            displayId={props.fact.displayId}
            presentation="compact"
          />
        );
      case "courseClassification":
        return <CourseClassificationSummary value={props.fact.value} />;
    }
  };

  return (
    <div
      class="record-list__fact"
      classList={{
        "record-list__fact--course-classification": props.fact.kind === "courseClassification",
      }}
    >
      <Show
        when={
          (props.fact.kind === "text" || props.fact.kind === "time") &&
          props.fact.label !== undefined
        }
      >
        <span class="record-list__fact-label">
          {props.fact.kind === "text" || props.fact.kind === "time" ? props.fact.label : ""}
        </span>
      </Show>
      {content()}
    </div>
  );
}

function RecordActionControl(props: { readonly action: Accessor<RecordAction> }): JSX.Element {
  const command = (): Extract<RecordAction, { readonly kind: "command" }> =>
    props.action() as Extract<RecordAction, { readonly kind: "command" }>;
  const link = (): Extract<RecordAction, { readonly kind: "link" }> =>
    props.action() as Extract<RecordAction, { readonly kind: "link" }>;

  return (
    <Show
      when={props.action().kind === "link"}
      fallback={
        <button
          class="record-list__action"
          classList={{ "record-list__action--primary": props.action().primary === true }}
          type="button"
          disabled={command().disabled}
          title={command().title}
          aria-pressed={command().pressed}
          aria-expanded={command().expanded}
          aria-controls={command().controls}
          ref={(element) => command().ref?.(element)}
          onClick={(event) => command().onClick(event)}
        >
          {props.action().label}
        </button>
      }
    >
      <a
        class="record-list__action"
        classList={{ "record-list__action--primary": link().primary === true }}
        href={link().href}
        title={link().title}
        ref={(element) => link().ref?.(element)}
        onClick={(event) => link().onFollow?.(event)}
      >
        {props.action().label}
      </a>
    </Show>
  );
}

function normalizeRecordActions(
  recordId: string,
  actions: ReadonlyArray<RecordAction>,
): { readonly ids: ReadonlyArray<string>; readonly byId: ReadonlyMap<string, RecordAction> } {
  const byId = new Map<string, RecordAction>();
  for (const action of actions) {
    if (byId.has(action.id)) {
      throw new Error(`RecordList record ${recordId} has duplicate action ID ${action.id}.`);
    }
    byId.set(action.id, action);
  }
  return { ids: [...byId.keys()], byId };
}

export function RecordSemanticContent(props: {
  readonly recordId: string;
  readonly content: Accessor<RecordContent>;
  readonly renderBody?: () => JSX.Element;
}): JSX.Element {
  const actions = createMemo(() => normalizeRecordActions(props.recordId, props.content().actions));
  const renderBody = props.renderBody;

  return (
    <div class="record-list__semantic-frame">
      <Show when={props.content().media}>
        {(media) => (
          <figure class="record-list__media">
            <img
              src={media().src}
              alt={media().alt}
              onLoad={(event) => {
                event.currentTarget.closest(".record-list__media")?.removeAttribute("hidden");
              }}
              onError={(event) => {
                event.currentTarget.closest(".record-list__media")?.setAttribute("hidden", "");
              }}
            />
          </figure>
        )}
      </Show>
      <div class="record-list__content">
        <h3 class="record-list__title">{props.content().title}</h3>
        <Show when={props.content().description}>
          {(description) => <p class="record-list__description">{description()}</p>}
        </Show>
        <Show when={props.content().details.length > 0}>
          <div class="record-list__facts">
            <For each={props.content().details}>{(fact) => <RecordFactView fact={fact} />}</For>
          </div>
        </Show>
        <Show when={props.content().actions.length > 0}>
          <div class="record-list__actions">
            <For each={actions().ids}>
              {(actionId) => <RecordActionControl action={() => actions().byId.get(actionId)!} />}
            </For>
          </div>
        </Show>
        {renderBody === undefined ? undefined : <div class="record-list__body">{renderBody()}</div>}
      </div>
    </div>
  );
}

function SemanticRecordRow<Row>(props: {
  readonly recordId: string;
  readonly row: Accessor<Row>;
  readonly content: (row: Row) => RecordContent;
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: RecordListSelection<Row>;
  readonly selectionName: string;
}): JSX.Element {
  const content = createMemo(() => props.content(props.row()));
  const body = props.renderBody;
  const renderBody = body === undefined ? undefined : (): JSX.Element => body(props.row);

  return (
    <article
      class="record-list__row record-list__row--semantic"
      classList={{ "record-list__row--selectable": props.selection !== undefined }}
      role="listitem"
      data-record-id={props.recordId}
    >
      <Show when={props.selection}>
        {(selection) => (
          <div class="record-list__selection">
            <label>
              <input
                type={selection().kind}
                name={selection().kind === "radio" ? props.selectionName : undefined}
                value={props.recordId}
                checked={selection().selectedIds().has(props.recordId)}
                disabled={selection().disabled?.(props.row())}
                onChange={(event) => selection().onChange(props.row(), event.currentTarget.checked)}
              />
              <span class="sr-only">Select {content().title}</span>
            </label>
          </div>
        )}
      </Show>
      <RecordSemanticContent recordId={props.recordId} content={content} renderBody={renderBody} />
    </article>
  );
}

/**
 * Renders semantic records and their shared loading, empty, and error states.
 * Presentation variants, reordering, and windowing compose outside this focused core.
 */
export function RecordList<Row>(props: RecordListProps<Row>): JSX.Element {
  const rowsById = createMemo(() => new Map(props.rows.map((row) => [props.recordId(row), row])));
  const recordIds = createMemo(() => props.rows.map((row) => props.recordId(row)));
  const selectionName = `record-list-selection-${createUniqueId()}`;

  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.rows.length === 0}
      emptyState={props.emptyState}
    >
      <div
        class="record-list record-list--semantic"
        classList={{ "record-list--gallery": props.presentation === "gallery" }}
        role="list"
        aria-label={props.ariaLabel}
      >
        <For each={recordIds()}>
          {(recordId) => (
            <SemanticRecordRow
              recordId={recordId}
              row={() => rowsById().get(recordId)!}
              content={props.content}
              renderBody={props.renderBody}
              selection={props.selection}
              selectionName={selectionName}
            />
          )}
        </For>
      </div>
    </RecordCollectionStateView>
  );
}
