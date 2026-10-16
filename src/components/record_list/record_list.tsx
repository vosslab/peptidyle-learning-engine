import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  mapArray,
  onCleanup,
  type Accessor,
  type JSX,
} from "solid-js";

import { assessmentTypePresentation } from "../../assessment_type_presentation";
import { CopyableQuestionId } from "../copyable_question_id";
import { CourseClassificationSummary } from "../course_classification_summary";
import { InstructorProfileLink } from "../instructor_profile_link";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { CourseClassification } from "../../../generated/api/CourseClassification";
import type { QuestionAuthor } from "../../../generated/api/QuestionAuthor";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import {
  recordIdentityField,
  type RecordField,
  type RecordFieldValue,
  type RecordListQuery,
} from "./record_fields";
import {
  defaultRecordActionsWidth,
  recordColumnWidth,
  visibleRecordFields,
  visibleTableWidth,
  type ColumnChoice,
} from "./record_column_visibility";
import { RecordTable, type RecordTableColumn, type RecordTableRowHeader } from "./record_table";
import { RecordActionControl } from "./record_action_control";
import { RecordFilterButton, RecordFilterControls } from "./record_filter_controls";
import { RecordSortHeading, RecordSortMenu, recordSortOptions } from "./record_sort_controls";
import {
  RecordListModeControls,
  RecordListSelectionControls,
  type RecordListMode,
  type RecordListSelectionToolbar,
} from "./record_list_controls";
import "./record_list.css";

export type {
  RecordField,
  RecordFieldScalar,
  RecordFieldValue,
  RecordFilterBinding,
  RecordFilterChoice,
  RecordFilterChoices,
  RecordListQuery,
  RecordSort,
  RecordSortBinding,
} from "./record_fields";

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
  | { readonly kind: "courseClassification"; readonly value: CourseClassification }
  | {
      readonly kind: "instructorProfile";
      readonly accountId: string;
      readonly displayName: string;
      readonly label?: string;
    }
  | {
      readonly kind: "questionAuthors";
      readonly authors: ReadonlyArray<QuestionAuthor>;
      readonly label?: string;
    };

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
  /** The domain owns selection changes, caps, and any editor-discard guard. */
  readonly toolbar?: RecordListSelectionToolbar;
};

export type RecordListProps<
  Row,
  SortKey extends string = string,
  FilterKey extends string = string,
> = {
  readonly rows: ReadonlyArray<Row>;
  readonly recordId: (row: Row) => string;
  readonly content: (row: Row) => RecordContent;
  /** Bounded caller-owned body under the shared identity. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: RecordListSelection<Row>;
  readonly state: RecordListState;
  readonly ariaLabel: string;
  readonly emptyState: RecordListEmptyState;
} & (
  | { readonly fields?: never; readonly query?: never }
  | {
      /** One field definition supplies every presentation, including initially empty results. */
      readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
      readonly query: RecordListQuery<SortKey, FilterKey>;
    }
);

function RecordFactView(props: { readonly fact: RecordFact }): JSX.Element {
  const content = (): JSX.Element => {
    switch (props.fact.kind) {
      case "text":
        return <span>{props.fact.value}</span>;
      case "time":
        return <time datetime={props.fact.dateTime}>{props.fact.value}</time>;
      case "link":
        return (
          <a href={props.fact.href} target="_blank" rel="noopener" onClick={props.fact.onFollow}>
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
      case "instructorProfile":
        return (
          <InstructorProfileLink
            accountId={props.fact.accountId}
            displayName={props.fact.displayName}
          />
        );
      case "questionAuthors":
        return (
          <For each={props.fact.authors}>
            {(author, index) => (
              <>
                {index() > 0 ? ", " : ""}
                <Show when={author.accountId} fallback={author.displayName}>
                  {(accountId) => (
                    <InstructorProfileLink
                      accountId={accountId()}
                      displayName={author.displayName}
                    />
                  )}
                </Show>
              </>
            )}
          </For>
        );
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
          (props.fact.kind === "text" ||
            props.fact.kind === "time" ||
            props.fact.kind === "instructorProfile" ||
            props.fact.kind === "questionAuthors") &&
          props.fact.label !== undefined
        }
      >
        <span class="record-list__fact-label">
          {props.fact.kind === "text" ||
          props.fact.kind === "time" ||
          props.fact.kind === "instructorProfile" ||
          props.fact.kind === "questionAuthors"
            ? props.fact.label
            : ""}
        </span>
      </Show>
      {content()}
    </div>
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

  function meaningfulDescription(): string | undefined {
    const { title, description } = props.content();
    if (description === undefined) return undefined;

    const trimmedDescription = description.trim();
    if (trimmedDescription.length === 0 || trimmedDescription === title.trim()) return undefined;

    return description;
  }

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
        <div class="record-list__identity">
          <h3 class="record-list__title">{props.content().title}</h3>
          <Show when={meaningfulDescription()}>
            {(description) => <p class="record-list__description">{description()}</p>}
          </Show>
        </div>
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

function fieldText(value: RecordFieldValue): string {
  if (value === null) return "";
  if (Array.isArray(value)) return value.map(String).join(", ");
  return String(value);
}

function CompactRecordIdentity<Row>(props: {
  readonly recordId: string;
  readonly row: Accessor<Row>;
  readonly content: (row: Row) => RecordContent;
  readonly renderBody?: () => JSX.Element;
  readonly selection?: RecordListSelection<Row>;
  readonly selectionName: string;
}): JSX.Element {
  const content = createMemo(() => props.content(props.row()));
  const renderBody = props.renderBody;

  return (
    <div class="record-list__compact-identity">
      <Show when={props.selection}>
        {(selection) => (
          <label class="record-list__compact-selection">
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
        )}
      </Show>
      <span class="record-list__compact-title" title={content().title}>
        {content().title}
      </span>
      {renderBody === undefined ? undefined : (
        <div class="record-list__compact-body">{renderBody()}</div>
      )}
    </div>
  );
}

function CompactRecordActions(props: {
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

/** Renders records with local display modes, compact-column controls, and shared collection states. */
export function RecordList<Row, SortKey extends string = string, FilterKey extends string = string>(
  props: RecordListProps<Row, SortKey, FilterKey>,
): JSX.Element {
  const rowsById = createMemo(() => new Map(props.rows.map((row) => [props.recordId(row), row])));
  const recordIds = createMemo(() => props.rows.map((row) => props.recordId(row)));
  const bodyModels = mapArray(recordIds, (recordId) => ({
    recordId,
    body: props.renderBody?.(() => rowsById().get(recordId)!),
  }));
  const bodiesById = createMemo(
    () => new Map(bodyModels().map((model) => [model.recordId, model.body])),
  );
  const selectionName = `record-list-selection-${createUniqueId()}`;
  const [columnWidths, setColumnWidths] = createSignal<ReadonlyMap<string, number>>(new Map());
  const [columnChoices, setColumnChoices] = createSignal<ReadonlyMap<string, ColumnChoice>>(
    new Map(),
  );
  const [columnOrder, setColumnOrder] = createSignal<ReadonlyArray<string>>([]);
  const [mode, setMode] = createSignal<RecordListMode>("compact");
  const [openFilterKey, setOpenFilterKey] = createSignal<FilterKey | null>(null);
  let filterTrigger: HTMLButtonElement | undefined;
  function openFilter(key: FilterKey, trigger: HTMLButtonElement): void {
    filterTrigger = trigger;
    setOpenFilterKey(() => key);
  }
  function closeFilter(): void {
    setOpenFilterKey(null);
    filterTrigger?.focus();
    filterTrigger = undefined;
  }
  const [availableWidth, setAvailableWidth] = createSignal(0);
  const [rootFontSize, setRootFontSize] = createSignal(16);
  const identityField = createMemo(() => recordIdentityField(props.fields ?? []));
  createEffect(() => {
    const available = new Set((props.fields ?? []).map((field) => field.id));
    setColumnOrder((current) => {
      const retained = current.filter((id) => available.has(id));
      return retained.length === current.length ? current : retained;
    });
  });
  const orderedFieldIds = createMemo((): ReadonlyArray<string> => {
    const declared = (props.fields ?? []).map((field) => field.id);
    const available = new Set(declared);
    const retained = columnOrder().filter((id) => available.has(id));
    const retainedIds = new Set(retained);
    return [...retained, ...declared.filter((id) => !retainedIds.has(id))];
  });
  function moveColumn(id: string, targetId: string): void {
    if (id === targetId) return;
    const next = [...orderedFieldIds()];
    const from = next.indexOf(id);
    const target = next.indexOf(targetId);
    if (from < 0 || target < 0) return;
    next.splice(from, 1);
    next.splice(target, 0, id);
    setColumnOrder(next);
  }
  function moveColumnBy(id: string, direction: -1 | 1): void {
    const order = orderedFieldIds();
    const index = order.indexOf(id);
    const neighbor = order[index + direction];
    if (neighbor !== undefined) moveColumn(id, neighbor);
  }
  const recordContent = (record: Row): RecordContent => {
    const content = props.content(record);
    const identity = identityField();
    return identity === undefined ? content : { ...content, title: identity.value(record) };
  };
  const hasRecordActions = createMemo(() =>
    props.rows.some((record) => props.content(record).actions.length > 0),
  );
  const defaultActionsWidth = createMemo(() =>
    defaultRecordActionsWidth(availableWidth(), rootFontSize()),
  );
  let widthObserver: ResizeObserver | undefined;
  onCleanup(() => widthObserver?.disconnect());
  const visibleFields = createMemo(() =>
    visibleRecordFields(
      props.fields ?? [],
      columnChoices(),
      availableWidth(),
      rootFontSize(),
      hasRecordActions(),
      recordColumnWidth(identityField()?.defaultColumn.width, rootFontSize()),
    ),
  );
  function chooseColumn(id: string, choice: ColumnChoice | "auto"): void {
    const next = new Map(columnChoices());
    if (choice === "auto") next.delete(id);
    else next.set(id, choice);
    setColumnChoices(next);
  }
  function observeCompactWidth(element: HTMLDivElement): void {
    widthObserver?.disconnect();
    setRootFontSize(Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
    widthObserver = new ResizeObserver(([entry]) => {
      if (entry) {
        setRootFontSize(
          Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16,
        );
        setAvailableWidth(entry.contentRect.width);
      }
    });
    widthObserver.observe(element);
  }
  const row = (recordId: string): Row => rowsById().get(recordId)!;
  const sortDirection = (key: SortKey | undefined): "ascending" | "descending" | undefined =>
    key !== undefined && props.query?.sort?.value?.key === key
      ? props.query.sort.value.direction
      : undefined;
  const compactSortSummary = (): string => {
    const binding = props.query?.sort;
    const sort = binding?.value;
    if (!binding || !sort) return "Sort";
    if (
      identityField()?.sortKey === sort.key ||
      visibleFields().some((field) => field.sortKey === sort.key)
    ) {
      return "Sort";
    }
    const label = recordSortOptions(props.fields ?? [], binding).find(
      (option) => option.sort.key === sort.key && option.sort.direction === sort.direction,
    )?.label;
    return label ? `Sort: ${label}` : "Sort";
  };
  const columns = createMemo((): ReadonlyArray<RecordTableColumn<string>> => {
    const dataColumns: ReadonlyArray<RecordTableColumn<string>> = visibleFields().map((field) => ({
      id: field.id,
      header: (
        <span class="record-filters__heading">
          <Show
            when={field.sortKey !== undefined && props.query?.sort !== undefined}
            fallback={field.label}
          >
            <RecordSortHeading
              label={field.label}
              sortKey={field.sortKey!}
              binding={props.query!.sort!}
            />
          </Show>
          <Show when={field.filterKey !== undefined && props.query?.filters}>
            <RecordFilterButton
              label={field.label}
              count={props.query?.filters?.values.get(field.filterKey!)?.length ?? 0}
              onOpen={(trigger) => openFilter(field.filterKey!, trigger)}
            />
          </Show>
        </span>
      ),
      sortDirection: () => sortDirection(field.sortKey),
      resizeLabel: field.label,
      width: field.defaultColumn.width,
      cell: (recordId: string): JSX.Element => (
        <span
          class="record-list__compact-value"
          title={field.render === undefined ? fieldText(field.value(row(recordId))) : undefined}
        >
          {field.render?.(row(recordId)) ?? fieldText(field.value(row(recordId)))}
        </span>
      ),
    }));

    return hasRecordActions()
      ? [
          ...dataColumns,
          {
            id: "record_actions",
            header: "Actions",
            resizeLabel: "Actions",
            minWidth: 112,
            width: `${defaultActionsWidth()}px`,
            cell: (recordId: string): JSX.Element => (
              <CompactRecordActions
                recordId={recordId}
                content={() => recordContent(row(recordId))}
              />
            ),
          },
        ]
      : dataColumns;
  });

  const rowHeader = (): RecordTableRowHeader<string> => ({
    id: identityField()?.id ?? "record_identity",
    header: (
      <span class="record-filters__heading">
        <Show
          when={identityField()?.sortKey !== undefined && props.query?.sort !== undefined}
          fallback={identityField()?.label ?? "Record"}
        >
          <RecordSortHeading
            label={identityField()?.label ?? "Record"}
            sortKey={identityField()!.sortKey!}
            binding={props.query!.sort!}
          />
        </Show>
        <Show when={identityField()?.filterKey !== undefined && props.query?.filters}>
          <RecordFilterButton
            label={identityField()?.label ?? "Record"}
            count={props.query?.filters?.values.get(identityField()!.filterKey!)?.length ?? 0}
            onOpen={(trigger) => openFilter(identityField()!.filterKey!, trigger)}
          />
        </Show>
      </span>
    ),
    sortDirection: () => sortDirection(identityField()?.sortKey),
    resizeLabel: identityField()?.label ?? "Record",
    minWidth: 160,
    width: identityField()?.defaultColumn.width ?? (props.fields?.length ? "30%" : "65%"),
    content: (recordId: string): JSX.Element => (
      <CompactRecordIdentity
        recordId={recordId}
        row={() => row(recordId)}
        content={recordContent}
        renderBody={props.renderBody ? (): JSX.Element => bodiesById().get(recordId) : undefined}
        selection={props.selection}
        selectionName={selectionName}
      />
    ),
  });
  const orderedColumnIds = (): ReadonlyArray<string> | undefined => {
    if (!props.fields?.length) return undefined;
    const shown = new Set(visibleFields().map((field) => field.id));
    return [
      ...(identityField() ? [] : ["record_identity"]),
      ...orderedFieldIds().filter((id) => id === identityField()?.id || shown.has(id)),
      ...(hasRecordActions() ? ["record_actions"] : []),
    ];
  };

  return (
    <div class="record-list__collection">
      <RecordListModeControls mode={mode} onChange={setMode} />
      <Show when={props.query?.sort}>
        {(binding) => (
          <Show
            when={mode() === "compact"}
            fallback={<RecordSortMenu fields={props.fields ?? []} binding={binding()} />}
          >
            <details class="record-sort__disclosure">
              <summary>{compactSortSummary()}</summary>
              <RecordSortMenu fields={props.fields ?? []} binding={binding()} />
            </details>
          </Show>
        )}
      </Show>
      <Show when={props.query?.filters}>
        {(binding) => (
          <RecordFilterControls
            fields={props.fields ?? []}
            binding={binding()}
            compact={mode() === "compact"}
            openKey={openFilterKey()}
            onOpen={openFilter}
            onClose={closeFilter}
          />
        )}
      </Show>
      <Show when={props.selection?.kind === "checkbox" && props.selection.toolbar}>
        <RecordListSelectionControls
          loadedCount={props.rows.length}
          selectedCount={() => props.selection!.selectedIds().size}
          toolbar={props.selection!.toolbar!}
        />
      </Show>
      <Show
        when={mode() === "compact"}
        fallback={
          <RecordCollectionStateView
            state={props.state}
            isEmpty={props.rows.length === 0}
            emptyState={props.emptyState}
          >
            <div
              class="record-list record-list--semantic"
              classList={{
                "record-list--gallery": mode() === "poster",
                "record-list--poster": mode() === "poster",
              }}
              role="list"
              aria-label={props.ariaLabel}
            >
              <For each={recordIds()}>
                {(recordId) => (
                  <SemanticRecordRow
                    recordId={recordId}
                    row={() => row(recordId)}
                    content={recordContent}
                    renderBody={
                      props.renderBody ? (): JSX.Element => bodiesById().get(recordId) : undefined
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
        <div class="record-list--compact" ref={observeCompactWidth}>
          <Show when={props.fields?.length}>
            <details class="record-list__column-controls">
              <summary>Columns</summary>
              <div class="record-list__column-controls-body">
                <For each={orderedFieldIds()}>
                  {(id) => {
                    const field = (): RecordField<Row, SortKey, FilterKey> | undefined =>
                      props.fields?.find((entry) => entry.id === id);
                    return (
                      <div class="record-list__column-move">
                        <span>{field()?.label}</span>
                        <button
                          type="button"
                          aria-label={`Move ${field()?.label} column left`}
                          disabled={orderedFieldIds().indexOf(id) === 0}
                          onClick={() => moveColumnBy(id, -1)}
                        >
                          &larr;
                        </button>
                        <button
                          type="button"
                          aria-label={`Move ${field()?.label} column right`}
                          disabled={orderedFieldIds().indexOf(id) === orderedFieldIds().length - 1}
                          onClick={() => moveColumnBy(id, 1)}
                        >
                          &rarr;
                        </button>
                      </div>
                    );
                  }}
                </For>
                <For each={props.fields?.filter((field) => field.role !== "identity")}>
                  {(field) => (
                    <label class="record-list__column-choice">
                      <span>{field.label}</span>
                      <select
                        aria-label={`${field.label} column visibility`}
                        value={columnChoices().get(field.id) ?? "auto"}
                        onChange={(event) =>
                          chooseColumn(field.id, event.currentTarget.value as ColumnChoice | "auto")
                        }
                      >
                        <option value="auto">Automatic</option>
                        <option value="show">Show</option>
                        <option value="hide">Hide</option>
                      </select>
                    </label>
                  )}
                </For>
                <button
                  type="button"
                  onClick={() => {
                    setColumnChoices(new Map());
                    setColumnWidths(new Map());
                    setColumnOrder([]);
                  }}
                >
                  Restore default columns
                </button>
              </div>
            </details>
          </Show>
          <RecordTable
            rows={recordIds()}
            columns={columns()}
            rowId={(recordId) => recordId}
            rowHeader={rowHeader()}
            orderedColumnIds={orderedColumnIds()}
            onColumnMove={moveColumn}
            movableColumnIds={orderedFieldIds()}
            columnWidths={columnWidths()}
            onColumnWidthsChange={setColumnWidths}
            minimumWidth={visibleTableWidth(
              visibleFields(),
              columnWidths(),
              availableWidth(),
              rootFontSize(),
              hasRecordActions(),
              rowHeader().id,
              recordColumnWidth(identityField()?.defaultColumn.width, rootFontSize()),
            )}
            state={props.state}
            ariaLabel={props.ariaLabel}
            emptyState={props.emptyState}
          />
        </div>
      </Show>
    </div>
  );
}
