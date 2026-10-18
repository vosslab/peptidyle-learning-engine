import { For, Show, type JSX } from "solid-js";

import type { SearchStateController } from "./search_state";
import type { SearchAppliedFilter } from "./search_session";
import "./search.css";

export type SearchControlsProps<Query, Row, FilterCounts> = {
  readonly state: SearchStateController<Query, Row, FilterCounts>;
  readonly textLabel: string;
  readonly textPlaceholder?: string;
  readonly maxlength?: number;
  readonly disabled?: () => boolean;
  /** Combines a content-owned local text field with the shared text before one request. */
  readonly onSubmit?: () => void;
  /** Clears content-owned local display state after the shared session returns to idle. */
  readonly onClear?: () => void;
  /** Reports content-owned draft state so Clear remains available before a first submit. */
  readonly hasDraft?: () => boolean;
  /** Content-specific fields stay beside the universal text and chips. */
  readonly children?: JSX.Element;
};

/** Shared submit boundary and query-independent search controls. */
export function SearchControls<Query, Row, FilterCounts>(
  props: SearchControlsProps<Query, Row, FilterCounts>,
): JSX.Element {
  const disabled = (): boolean => props.disabled?.() === true;
  const loading = (): boolean => props.state.state().kind === "loading";
  const filters = (): ReadonlyArray<SearchAppliedFilter<Query>> =>
    props.state.definition.appliedFilters?.(props.state.query()) ?? [];
  const searched = (): boolean => props.state.state().kind !== "initial";
  const hasDraft = (): boolean => props.hasDraft?.() === true;
  const canClear = (): boolean =>
    searched() || props.state.searchUsed() || props.state.typedText().length > 0 || hasDraft();
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
          maxlength={props.maxlength}
          disabled={disabled()}
          onInput={(event) => props.state.setTypedText(event.currentTarget.value)}
        />
      </label>
      <button type="submit" class="primary-action" disabled={disabled()}>
        Search
      </button>
      <Show when={searched()}>
        <button
          type="button"
          class="quiet-action"
          disabled={disabled() || loading()}
          onClick={() => void props.state.refresh()}
        >
          Refresh
        </button>
        <Show when={appliedText().length > 0 && appliedText() !== props.state.typedText().trim()}>
          <p class="shared-search-controls__applied">Applied search: {appliedText()}</p>
        </Show>
      </Show>
      <Show when={canClear()}>
        <button
          type="button"
          class="quiet-action"
          disabled={disabled()}
          onClick={() => {
            void props.state.clear().then((cleared) => {
              if (!cleared) return;
              props.onClear?.();
              queueMicrotask(() => searchInput?.focus());
            });
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
                onClick={() => void props.state.apply((current) => filter.clear(current))}
              >
                {filter.label}
              </button>
            )}
          </For>
        </div>
      </Show>
      {props.children}
    </form>
  );
}
