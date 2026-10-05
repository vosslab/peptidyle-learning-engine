import { For, Show, type JSX } from "solid-js";

import { RecordSortControl } from "../../components/record_list/record_sort_control";
import { SearchResultDisplay } from "../../components/search_result_display";
import type { SearchStateController } from "./search_state";
import type { SearchAppliedFilter } from "./search_session";
import "./search.css";

export type SearchControlsProps<Query, Row, FilterCounts, Sort extends string = never> = {
  readonly state: SearchStateController<Query, Row, FilterCounts, Sort>;
  readonly textLabel: string;
  readonly textPlaceholder?: string;
  readonly displayAriaLabel?: string;
  readonly disabled?: () => boolean;
  /** Combines a content-owned local text field with the shared text before one request. */
  readonly onSubmit?: () => void;
  /** Clears content-owned local display state after the shared session returns to idle. */
  readonly onClear?: () => void;
  /** Reports content-owned draft state so Clear remains available before a first submit. */
  readonly hasDraft?: () => boolean;
  /** Content-specific fields stay beside the universal text, chips, sort, and display controls. */
  readonly children?: JSX.Element;
};

/** Shared submit boundary and query-independent search controls. */
export function SearchControls<Query, Row, FilterCounts, Sort extends string = never>(
  props: SearchControlsProps<Query, Row, FilterCounts, Sort>,
): JSX.Element {
  const disabled = (): boolean => props.disabled?.() === true;
  const filters = (): ReadonlyArray<SearchAppliedFilter<Query>> =>
    props.state.definition.appliedFilters?.(props.state.query()) ?? [];
  const searched = (): boolean => props.state.state().kind !== "initial";
  const hasDraft = (): boolean => props.hasDraft?.() === true;
  const canClear = (): boolean => searched() || props.state.typedText().length > 0 || hasDraft();
  const appliedText = (): string => props.state.definition.getText(props.state.query());
  let searchInput: HTMLInputElement | undefined;

  return (
    <form
      class="shared-search-controls"
      classList={{ "shared-search-controls--initial": !searched() }}
      onSubmit={(event) => {
        event.preventDefault();
        if (props.onSubmit !== undefined) props.onSubmit();
        else void props.state.submit();
      }}
    >
      <label class="shared-search-controls__text">
        {props.textLabel}
        <input
          type="search"
          ref={(element) => (searchInput = element)}
          value={props.state.typedText()}
          placeholder={props.textPlaceholder}
          disabled={disabled()}
          onInput={(event) => props.state.setTypedText(event.currentTarget.value)}
        />
      </label>
      <button type="submit" class="primary-action" disabled={disabled()}>
        Search
      </button>
      <Show when={searched()}>
        <p class="shared-search-controls__summary" aria-live="polite">
          {props.state.state().rows.length} shown on this page
        </p>
        <Show when={appliedText().length > 0}>
          <p class="shared-search-controls__applied">Applied search: {appliedText()}</p>
        </Show>
      </Show>
      <Show when={canClear()}>
        <button
          type="button"
          class="quiet-action"
          disabled={disabled()}
          onClick={() => {
            props.state.clear();
            props.onClear?.();
            queueMicrotask(() => searchInput?.focus());
          }}
        >
          Clear all
        </button>
      </Show>
      <Show when={filters().length > 0}>
        <div class="shared-search-controls__chips" aria-label="Applied filters">
          <For each={filters()}>
            {(filter) => (
              <button
                type="button"
                class="shared-search-controls__chip"
                disabled={disabled()}
                aria-label={`Remove ${filter.label}`}
                onClick={() => void props.state.apply(filter.clear(props.state.query()))}
              >
                {filter.label}
              </button>
            )}
          </For>
        </div>
      </Show>
      {props.children}
      <Show when={searched() && props.state.definition.sort !== undefined}>
        <RecordSortControl
          label={props.state.definition.sort!.label}
          options={props.state.definition.sort!.options}
          value={props.state.definition.sort!.get(props.state.query())}
          disabled={disabled()}
          onChange={(value) =>
            void props.state.apply(props.state.definition.sort!.set(props.state.query(), value))
          }
        />
      </Show>
      <Show when={searched()}>
        <SearchResultDisplay
          ariaLabel={props.displayAriaLabel ?? "Search result display"}
          mode={props.state.display}
          onChange={props.state.setDisplay}
        />
      </Show>
    </form>
  );
}
