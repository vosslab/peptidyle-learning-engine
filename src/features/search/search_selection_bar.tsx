import { Show, type JSX } from "solid-js";
import "./search.css";

export type SearchSelectionBarProps = {
  readonly selectedCount: () => number;
  readonly loadedCount: () => number;
  readonly disabled?: () => boolean;
  readonly onSelectLoaded: () => void;
  readonly onClearSelection: () => void;
  readonly children?: JSX.Element;
};

/** Common selection status and page-local selection commands; actions remain content-owned. */
export function SearchSelectionBar(props: SearchSelectionBarProps): JSX.Element {
  return (
    <Show when={props.loadedCount() > 0 || props.selectedCount() > 0}>
      <section class="shared-search-selection" aria-label="Selected search results">
        <p aria-live="polite">
          <strong>{props.selectedCount()} selected</strong> from {props.loadedCount()} loaded
          results
        </p>
        <div>
          <button
            type="button"
            class="quiet-action"
            disabled={props.disabled?.()}
            onClick={props.onSelectLoaded}
          >
            Select loaded results
          </button>
          <button
            type="button"
            class="quiet-action"
            disabled={props.disabled?.() || props.selectedCount() === 0}
            onClick={props.onClearSelection}
          >
            Clear selection
          </button>
          {props.children}
        </div>
      </section>
    </Show>
  );
}
