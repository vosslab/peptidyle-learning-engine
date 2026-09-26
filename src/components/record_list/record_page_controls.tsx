import { For, type JSX } from "solid-js";

import "./record_family.css";

export const RECORD_PAGE_SIZES = [50, 100, 250] as const;
export type RecordPageSize = (typeof RECORD_PAGE_SIZES)[number];

type RecordPageSizeControlProps = {
  /** Caller-owned selected discovery page size. */
  readonly pageSize: RecordPageSize;
  /** Receives a fixed page-size choice; the caller starts its new cursor sequence. */
  readonly onPageSizeChange: (pageSize: RecordPageSize) => void;
};

type WithoutRecordPageSizeControlProps = {
  readonly pageSize?: never;
  readonly onPageSizeChange?: never;
};

type RecordPageControlsSharedProps = {
  /** Names this navigation landmark for its owning discovery or task list. */
  readonly ariaLabel: string;
  readonly hasPrevious: boolean;
  readonly hasNext: boolean;
  readonly loading?: boolean;
  readonly disabled?: boolean;
  readonly onPrevious: () => void;
  readonly onNext: () => void;
};

/**
 * Caller-controlled cursor navigation with an optional fixed discovery page-size choice.
 * It owns no cursor, result rows, totals, fetches, or page-reset policy.
 */
export type RecordPageControlsProps = RecordPageControlsSharedProps &
  (RecordPageSizeControlProps | WithoutRecordPageSizeControlProps);

/** Native Previous/Next controls for one bounded record collection. */
export function RecordPageControls(props: RecordPageControlsProps): JSX.Element {
  const actionsDisabled = (): boolean => props.loading === true || props.disabled === true;

  function pageSizeControl(): JSX.Element | undefined {
    if (props.pageSize === undefined || props.onPageSizeChange === undefined) return undefined;
    const onPageSizeChange = props.onPageSizeChange;
    return (
      <label class="record-page-controls__size">
        <span>Records per page</span>
        <select
          value={props.pageSize}
          disabled={actionsDisabled()}
          onChange={(event): void => {
            const pageSize = Number(event.currentTarget.value);
            const selected = RECORD_PAGE_SIZES.find((size) => size === pageSize);
            if (selected !== undefined) onPageSizeChange(selected);
          }}
        >
          <For each={RECORD_PAGE_SIZES}>{(size) => <option value={size}>{size}</option>}</For>
        </select>
      </label>
    );
  }

  return (
    <nav
      class="record-page-controls"
      aria-label={props.ariaLabel}
      aria-busy={props.loading === true}
    >
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
      {pageSizeControl()}
    </nav>
  );
}
