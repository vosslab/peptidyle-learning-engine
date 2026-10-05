// search_result_display.tsx - shared local display selector for searchable record collections.

import { For, type Accessor, type JSX } from "solid-js";

export type SearchResultDisplayMode = "compact" | "list" | "poster";

export interface SearchResultDisplayProps {
  readonly ariaLabel: string;
  readonly mode: Accessor<SearchResultDisplayMode>;
  readonly onChange: (mode: SearchResultDisplayMode) => void;
}

const MODES: ReadonlyArray<{
  readonly id: SearchResultDisplayMode;
  readonly label: string;
  readonly tooltip: string;
}> = [
  { id: "compact", label: "Compact", tooltip: "Dense rows for fast comparison" },
  { id: "list", label: "List", tooltip: "Expanded rows with summaries" },
  { id: "poster", label: "Visual boxes", tooltip: "Image-friendly boxes for visual content" },
];

/** A reusable, local-only choice; searches never save a display preference. */
export function SearchResultDisplay(props: SearchResultDisplayProps): JSX.Element {
  return (
    <div class="search-result-display" role="group" aria-label={props.ariaLabel}>
      <span>Display</span>
      <For each={MODES}>
        {(option) => (
          <button
            class="quiet-action"
            type="button"
            title={option.tooltip}
            aria-pressed={props.mode() === option.id}
            onClick={() => props.onChange(option.id)}
          >
            {option.label}
          </button>
        )}
      </For>
    </div>
  );
}
