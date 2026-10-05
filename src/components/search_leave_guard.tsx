// search_leave_guard.tsx - protects a completed search from accidental same-tab navigation.

import { useBeforeLeave, type BeforeLeaveEventArgs } from "@solidjs/router";
import {
  Show,
  createSignal,
  createUniqueId,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
} from "solid-js";

export interface SearchLeaveGuardProps {
  /** True after this page has a search result set that current-tab navigation would discard. */
  readonly hasSearch: Accessor<boolean>;
}

/** Confirms only for same-tab navigation away from a completed search. */
export function SearchLeaveGuard(props: SearchLeaveGuardProps): JSX.Element {
  const [pendingLeave, setPendingLeave] = createSignal<BeforeLeaveEventArgs>();
  const titleId = `search-leave-title-${createUniqueId()}`;
  const descriptionId = `search-leave-description-${createUniqueId()}`;
  let dialog: HTMLDialogElement | undefined;
  let stayButton: HTMLButtonElement | undefined;
  let returnFocus: HTMLElement | undefined;

  function dismiss(): void {
    if (dialog?.open) dialog.close();
    setPendingLeave(undefined);
    queueMicrotask(() => returnFocus?.focus());
  }

  useBeforeLeave((event) => {
    if (!props.hasSearch() || event.defaultPrevented || pendingLeave() !== undefined) return;
    event.preventDefault();
    returnFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    setPendingLeave(event);
  });

  onMount(() => {
    // ASVS 3.7.1: no search request or result is written to browser storage.
    const beforeUnload = (event: BeforeUnloadEvent): void => {
      if (!props.hasSearch()) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", beforeUnload);
    onCleanup(() => window.removeEventListener("beforeunload", beforeUnload));
  });

  return (
    <Show when={pendingLeave() !== undefined}>
      <dialog
        class="confirmation-dialog"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        ref={(element) => {
          dialog = element;
          queueMicrotask(() => {
            element.showModal();
            stayButton?.focus();
          });
        }}
        onCancel={(event) => {
          event.preventDefault();
          dismiss();
        }}
      >
        <h2 id={titleId}>Leave this search?</h2>
        <p id={descriptionId}>Your current search and its results will be discarded.</p>
        <div class="action-row">
          <button
            ref={(element) => (stayButton = element)}
            class="quiet-action"
            type="button"
            onClick={dismiss}
          >
            Stay on page
          </button>
          <button
            class="primary-action"
            type="button"
            onClick={() => {
              const leave = pendingLeave();
              if (dialog?.open) dialog.close();
              setPendingLeave(undefined);
              leave?.retry(true);
            }}
          >
            Leave page
          </button>
        </div>
      </dialog>
    </Show>
  );
}
