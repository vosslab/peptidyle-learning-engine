import { For, Show, type JSX } from "solid-js";
import { RECORD_PAGE_SIZES, type RecordPageSize } from "./record_page_sizes";

import "./record_family.css";

export { RECORD_PAGE_SIZES, type RecordPageSize } from "./record_page_sizes";

type RecordPageSizeControlProps = {
  /** Caller-owned selected page size. */
  readonly pageSize: RecordPageSize;
  /** Largest page size accepted by the owning query, defaulting to the shared maximum. */
  readonly maximumPageSize?: RecordPageSize;
  /** Receives a fixed page-size choice; the query owner preserves the first visible record. */
  readonly onPageSizeChange: (pageSize: RecordPageSize) => void;
};

type RecordPageControlsSharedProps = {
  /** Names this navigation landmark for its owning discovery or task list. */
  readonly ariaLabel: string;
  /** One-based page number resolved by the query owner. */
  readonly pageNumber: number;
  /** Complete matching result count, after scope, filters, and ordering. */
  readonly totalCount: number;
  readonly hasPrevious: boolean;
  readonly hasNext: boolean;
  readonly loading?: boolean;
  readonly disabled?: boolean;
  readonly onPrevious: () => void;
  readonly onNext: () => void;
  readonly onGoToPage: (pageNumber: number) => void;
};

/**
 * Caller-controlled numbered navigation and fixed page-size settings.
 * It owns no query, result rows, totals, fetches, or page-reset policy.
 */
export type RecordPageControlsProps = RecordPageControlsSharedProps & RecordPageSizeControlProps;

type PageToken = number | "ellipsis";

function visiblePageNumbers(pageNumber: number, totalPages: number): ReadonlyArray<PageToken> {
  const candidates = [1, pageNumber - 1, pageNumber, pageNumber + 1, totalPages];
  const pages = [
    ...new Set(candidates.filter((candidate) => candidate >= 1 && candidate <= totalPages)),
  ].sort((left, right) => left - right);
  const tokens: PageToken[] = [];
  for (const page of pages) {
    const previous = tokens[tokens.length - 1];
    if (typeof previous === "number" && page > previous + 1) tokens.push("ellipsis");
    tokens.push(page);
  }
  return tokens;
}

/** Fixed page-size choice composed inside DataSheet Options. */
export function RecordPageSizeControl(
  props: RecordPageSizeControlProps & Pick<RecordPageControlsSharedProps, "loading" | "disabled">,
): JSX.Element {
  const actionsDisabled = (): boolean => props.loading === true || props.disabled === true;
  const maximumPageSize = (): RecordPageSize =>
    props.maximumPageSize ?? RECORD_PAGE_SIZES[RECORD_PAGE_SIZES.length - 1]!;
  return (
    <label class="record-page-controls__size">
      <span>Records per page</span>
      <select
        value={props.pageSize}
        disabled={actionsDisabled()}
        onChange={(event): void => {
          const pageSize = Number(event.currentTarget.value);
          const selected = RECORD_PAGE_SIZES.find((size) => size === pageSize);
          if (selected !== undefined) props.onPageSizeChange(selected);
        }}
      >
        <For each={RECORD_PAGE_SIZES.filter((size) => size <= maximumPageSize())}>
          {(size) => <option value={size}>{size}</option>}
        </For>
      </select>
    </label>
  );
}

/** Native direct-page, Previous/Next controls for one bounded record collection. */
export function RecordPageControls(props: RecordPageControlsProps): JSX.Element {
  const actionsDisabled = (): boolean => props.loading === true || props.disabled === true;
  const totalPages = (): number => Math.max(1, Math.ceil(props.totalCount / props.pageSize));

  return (
    <nav
      class="record-page-controls"
      aria-label={props.ariaLabel}
      aria-busy={props.loading === true}
    >
      <span class="record-page-controls__status" aria-live="polite">
        <Show when={totalPages() > 1} fallback={`${props.totalCount} records`}>
          Page {props.pageNumber} of {totalPages()} ({props.totalCount} records)
        </Show>
      </span>
      <Show when={totalPages() > 1}>
        <button
          type="button"
          disabled={actionsDisabled() || !props.hasPrevious}
          onClick={props.onPrevious}
        >
          Previous
        </button>
        <button type="button" disabled={actionsDisabled() || !props.hasNext} onClick={props.onNext}>
          Next
        </button>
        <details class="record-page-controls__details">
          <summary>Pages</summary>
          <div class="record-page-controls__details-body">
            <For each={visiblePageNumbers(props.pageNumber, totalPages())}>
              {(token) =>
                token === "ellipsis" ? (
                  <span aria-hidden="true">...</span>
                ) : (
                  <button
                    type="button"
                    aria-current={token === props.pageNumber ? "page" : undefined}
                    disabled={actionsDisabled() || token === props.pageNumber}
                    onClick={() => props.onGoToPage(token)}
                  >
                    {token}
                  </button>
                )
              }
            </For>
            <Show when={totalPages() > 5}>
              <form
                class="record-page-controls__jump"
                onSubmit={(event): void => {
                  event.preventDefault();
                  const pageNumber = Number(new FormData(event.currentTarget).get("pageNumber"));
                  if (Number.isSafeInteger(pageNumber) && pageNumber >= 1)
                    props.onGoToPage(pageNumber);
                }}
              >
                <label>
                  <span>Go to page</span>
                  <input
                    name="pageNumber"
                    type="number"
                    min="1"
                    max={totalPages()}
                    value={props.pageNumber}
                    disabled={actionsDisabled()}
                  />
                </label>
                <button type="submit" disabled={actionsDisabled()}>
                  Go
                </button>
              </form>
            </Show>
          </div>
        </details>
      </Show>
    </nav>
  );
}
