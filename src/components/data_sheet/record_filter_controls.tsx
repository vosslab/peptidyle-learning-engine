import { For, Show, createEffect, createMemo, createSignal, onCleanup, type JSX } from "solid-js";

import { checkboxChange } from "../controlled_form_change";
import type { RecordField, RecordFilterBinding, RecordFilterChoice } from "./record_fields";
import "./record_filter_controls.css";

type FilterField = { readonly id: string; readonly label: string; readonly filterKey: string };
type ChoiceState<FilterKey extends string> = {
  readonly key: FilterKey;
  readonly choices: ReadonlyArray<RecordFilterChoice>;
  readonly selected: ReadonlyArray<RecordFilterChoice>;
  readonly nextCursor: string | null;
  readonly unavailableReason?: string;
};

export function RecordFilterButton(props: {
  readonly label: string;
  readonly count: number;
  readonly onOpen: (trigger: HTMLButtonElement) => void;
}): JSX.Element {
  return (
    <button
      type="button"
      class="record-filters__heading-button"
      aria-label={`Filter ${props.label}${props.count ? `, ${props.count} selected` : ""}`}
      aria-haspopup="dialog"
      onClick={(event) => props.onOpen(event.currentTarget)}
      onDragStart={(event) => event.preventDefault()}
    >
      Filter{props.count ? ` (${props.count})` : ""}
    </button>
  );
}

/** One picker and one common entry point across every RecordList presentation. */
export function RecordFilterControls<Row, SortKey extends string, FilterKey extends string>(props: {
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  readonly binding: RecordFilterBinding<FilterKey>;
  readonly compact: boolean;
  readonly openKey: FilterKey | null;
  readonly onOpen: (key: FilterKey, trigger: HTMLButtonElement) => void;
  readonly onClose: () => void;
}): JSX.Element {
  const filterFields = createMemo((): ReadonlyArray<FilterField> =>
    props.fields.flatMap((field) =>
      field.filterKey === undefined
        ? []
        : [{ id: field.id, label: field.label, filterKey: field.filterKey }],
    ),
  );
  const activeField = createMemo(() =>
    filterFields().find((field) => field.filterKey === props.openKey),
  );
  const selectedCount = createMemo(() =>
    filterFields().reduce(
      (count, field) =>
        count + (props.binding.values.get(field.filterKey as FilterKey)?.length ?? 0),
      0,
    ),
  );
  const selectedKeys = (): ReadonlyArray<string> =>
    props.openKey === null ? [] : (props.binding.values.get(props.openKey) ?? []);
  const [search, setSearch] = createSignal("");
  const [choiceState, setChoiceState] = createSignal<ChoiceState<FilterKey> | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal(false);
  const [retry, setRetry] = createSignal(0);
  let dialog!: HTMLDialogElement;
  let activeRequest: AbortController | undefined;
  let requestNumber = 0;

  function load(cursor: string | null, append: boolean): void {
    const key = props.openKey;
    if (key === null) return;
    activeRequest?.abort();
    const request = new AbortController();
    activeRequest = request;
    const number = ++requestNumber;
    setLoading(true);
    setError(false);
    void props.binding.choices(key, search(), cursor, request.signal).then(
      (result) => {
        if (request.signal.aborted || number !== requestNumber) return;
        const previous = choiceState();
        const byKey = new Map<string, RecordFilterChoice>();
        for (const choice of append && previous?.key === key ? previous.choices : []) {
          byKey.set(choice.key, choice);
        }
        for (const choice of result.choices) byKey.set(choice.key, choice);
        setChoiceState({
          key,
          choices: [...byKey.values()],
          selected: result.selected,
          nextCursor: result.nextCursor,
          unavailableReason: result.unavailableReason,
        });
        setLoading(false);
      },
      () => {
        if (request.signal.aborted || number !== requestNumber) return;
        setError(true);
        setLoading(false);
      },
    );
  }

  createEffect(() => {
    const key = props.openKey;
    const query = search();
    retry();
    if (key === null) return;
    void query;
    setChoiceState((previous) => ({
      key,
      choices: [],
      selected: previous?.key === key ? previous.selected : [],
      nextCursor: null,
      unavailableReason: undefined,
    }));
    load(null, false);
    onCleanup(() => {
      activeRequest?.abort();
      requestNumber++;
    });
  });

  createEffect(() => {
    if (props.openKey !== null && activeField()) {
      if (!dialog.open) dialog.showModal();
    } else if (dialog.open) {
      dialog.close();
    }
  });

  const visibleChoices = createMemo(() => {
    // ASVS 1.2.1: Solid renders provider labels as text in the option, not HTML.
    const state = choiceState();
    if (state?.key !== props.openKey) return [];
    const byKey = new Map<string, RecordFilterChoice>();
    for (const choice of state.selected) byKey.set(choice.key, choice);
    for (const choice of state.choices) byKey.set(choice.key, choice);
    return [...byKey.values()];
  });

  const nextCursor = (): string | null => {
    const state = choiceState();
    return state?.key === props.openKey ? state.nextCursor : null;
  };

  const unavailableReason = (): string | undefined => {
    const state = choiceState();
    return state?.key === props.openKey ? state.unavailableReason : undefined;
  };

  function choose(key: string, checked: boolean): void {
    const field = activeField();
    if (!field) return;
    const current = selectedKeys();
    props.binding.onChange(
      field.filterKey as FilterKey,
      checked ? [...new Set([...current, key])] : current.filter((entry) => entry !== key),
    );
  }

  function fieldButtons(): JSX.Element {
    return (
      <For each={filterFields()}>
        {(field) => {
          const count = (): number =>
            props.binding.values.get(field.filterKey as FilterKey)?.length ?? 0;
          return (
            <button
              type="button"
              aria-label={`Filter ${field.label}${count() ? `, ${count()} selected` : ""}`}
              aria-haspopup="dialog"
              onClick={(event) => props.onOpen(field.filterKey as FilterKey, event.currentTarget)}
            >
              {field.label}
              {count() ? ` (${count()})` : ""}
            </button>
          );
        }}
      </For>
    );
  }

  return (
    <>
      <div class="record-filters__toolbar" role="group" aria-label="Record filters">
        <Show
          when={props.compact}
          fallback={
            <>
              <span>Filters</span>
              <div class="record-filters__fields">{fieldButtons()}</div>
            </>
          }
        >
          <details class="record-filters__disclosure">
            <summary>Filters{selectedCount() ? ` (${selectedCount()})` : ""}</summary>
            <div class="record-filters__fields">{fieldButtons()}</div>
          </details>
        </Show>
        <Show when={selectedCount() > 0}>
          <button type="button" onClick={() => props.binding.onClear()}>
            Clear column filters
          </button>
        </Show>
      </div>
      <dialog
        ref={(element) => {
          dialog = element;
        }}
        class="record-filters__dialog"
        aria-label={activeField() ? `Filter ${activeField()!.label}` : "Record filter"}
        onClose={() => {
          props.onClose();
          setSearch("");
        }}
        onCancel={(event) => {
          event.preventDefault();
          dialog.close();
        }}
        onKeyDown={(event) => {
          if (event.key !== "Escape") return;
          event.preventDefault();
          event.stopPropagation();
          dialog.close();
        }}
      >
        <div class="record-filters__dialog-content">
          <div class="record-filters__dialog-heading">
            <strong>{activeField()?.label}</strong>
            <button type="button" onClick={() => dialog.close()} aria-label="Close filter">
              Close
            </button>
          </div>
          <label>
            Search values
            <input
              type="search"
              value={search()}
              onInput={(event) => setSearch(event.currentTarget.value)}
            />
          </label>
          <p aria-live="polite">
            {selectedKeys().length} selected{loading() ? ", loading values" : ""}
          </p>
          <Show when={error()}>
            {/* ASVS 16.5.1: keep provider error details out of the user-facing dialog. */}
            <p role="alert">Could not load values.</p>
            <button type="button" onClick={() => setRetry((value) => value + 1)}>
              Retry
            </button>
          </Show>
          <div class="record-filters__options">
            <For each={visibleChoices()}>
              {(choice) => (
                <label>
                  <input
                    type="checkbox"
                    checked={selectedKeys().includes(choice.key)}
                    onChange={(event) =>
                      checkboxChange(
                        event.currentTarget,
                        selectedKeys().includes(choice.key),
                        (checked) => choose(choice.key, checked),
                      )
                    }
                  />
                  <span>{choice.label}</span>
                  <span class="record-filters__count">{choice.count}</span>
                </label>
              )}
            </For>
          </div>
          <Show when={!loading() && !error() && unavailableReason()}>
            {(reason) => <p>{reason()}</p>}
          </Show>
          <Show
            when={!loading() && !error() && !unavailableReason() && visibleChoices().length === 0}
          >
            <p>No values found.</p>
          </Show>
          <Show when={nextCursor() !== null && !error()}>
            <button type="button" disabled={loading()} onClick={() => load(nextCursor(), true)}>
              Load more values
            </button>
          </Show>
        </div>
      </dialog>
    </>
  );
}
