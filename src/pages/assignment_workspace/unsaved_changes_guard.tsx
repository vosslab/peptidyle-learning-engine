import { useBeforeLeave, type BeforeLeaveEventArgs } from "@solidjs/router";
import { Show, createSignal, onCleanup, onMount, type Accessor, type JSX } from "solid-js";

interface UnsavedChangesGuardProps {
  readonly dirty: Accessor<boolean>;
  /** Returns true only after the current structural edit is safely persisted. */
  readonly save: () => Promise<boolean>;
}

/** Guards browser navigation while the Assignment Question Editor has local structural edits. */
export function UnsavedChangesGuard(props: UnsavedChangesGuardProps): JSX.Element {
  const [pendingLeave, setPendingLeave] = createSignal<BeforeLeaveEventArgs>();
  const [saving, setSaving] = createSignal(false);
  const [saveFailed, setSaveFailed] = createSignal(false);
  let dialog: HTMLDialogElement | undefined;
  let returnFocus: HTMLElement | undefined;
  let stayButton: HTMLButtonElement | undefined;

  function closePrompt(returnToTrigger = true): void {
    if (dialog?.open) dialog.close();
    setPendingLeave(undefined);
    setSaveFailed(false);
    if (returnToTrigger) queueMicrotask(() => returnFocus?.focus());
  }

  // Navigation resumes only after a successful Save or deliberate Discard.
  useBeforeLeave((event) => {
    if (!props.dirty() || event.defaultPrevented) return;
    event.preventDefault();
    if (pendingLeave() !== undefined) return;
    returnFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    setPendingLeave(event);
  });

  onMount(() => {
    const warnBeforeUnload = (event: BeforeUnloadEvent): void => {
      if (!props.dirty()) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warnBeforeUnload);
    onCleanup(() => window.removeEventListener("beforeunload", warnBeforeUnload));
  });

  async function saveAndContinue(): Promise<void> {
    setSaving(true);
    setSaveFailed(false);
    try {
      if (!(await props.save())) {
        setSaveFailed(true);
        return;
      }
      continueNavigation();
    } finally {
      setSaving(false);
    }
  }

  function continueNavigation(): void {
    const leave = pendingLeave();
    closePrompt(false);
    leave?.retry(true);
  }

  return (
    <Show when={pendingLeave()}>
      <dialog
        class="confirmation-dialog"
        aria-labelledby="unsaved-assignment-questions-heading"
        aria-describedby="unsaved-assignment-questions-copy"
        ref={(element) => {
          dialog = element;
          queueMicrotask(() => {
            element.showModal();
            stayButton?.focus();
          });
        }}
        onCancel={(event) => {
          event.preventDefault();
          closePrompt();
        }}
      >
        <h2 id="unsaved-assignment-questions-heading">Save Assignment Question changes?</h2>
        <p id="unsaved-assignment-questions-copy">
          Your Question order, additions, removals, or title changes have not been saved.
        </p>
        <Show when={saveFailed()}>
          <p role="status">
            Questions were not saved. Resolve the save error shown on the page, then try again or
            stay here.
          </p>
        </Show>
        <div class="action-row">
          <button
            ref={(element) => (stayButton = element)}
            class="quiet-action"
            type="button"
            disabled={saving()}
            onClick={() => closePrompt()}
          >
            Stay and keep editing
          </button>
          <button type="button" disabled={saving()} onClick={() => void saveAndContinue()}>
            {saving() ? "Saving Questions..." : "Save and continue"}
          </button>
          <button
            class="primary-action"
            type="button"
            disabled={saving()}
            onClick={continueNavigation}
          >
            Discard and continue
          </button>
        </div>
      </dialog>
    </Show>
  );
}
