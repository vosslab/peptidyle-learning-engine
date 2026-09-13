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

export interface UnsavedChangesGuardCopy {
  readonly heading: string;
  readonly description: JSX.Element;
  readonly saveActionLabel: string;
  readonly savingActionLabel: string;
  readonly saveFailureMessage: JSX.Element;
}

export interface UnsavedChangesGuardProps {
  readonly dirty: Accessor<boolean>;
  /** Returns true only after the local edits are safely persisted. */
  readonly save: () => Promise<boolean>;
  readonly copy: UnsavedChangesGuardCopy;
  /** Optional component-owned leave action, such as closing an editor dialog. */
  readonly manualLeave?: {
    readonly requested: Accessor<boolean>;
    readonly clearRequest: () => void;
    readonly continue: () => void;
  };
}

/**
 * Prevents route and browser unload navigation while an editor has unsaved local work.
 * Navigation resumes only after a successful Save or a deliberate Discard.
 */
export function UnsavedChangesGuard(props: UnsavedChangesGuardProps): JSX.Element {
  const [pendingLeave, setPendingLeave] = createSignal<BeforeLeaveEventArgs>();
  const [saving, setSaving] = createSignal(false);
  const [saveFailed, setSaveFailed] = createSignal(false);
  const headingId = `unsaved-changes-heading-${createUniqueId()}`;
  const descriptionId = `unsaved-changes-copy-${createUniqueId()}`;
  let dialog: HTMLDialogElement | undefined;
  let returnFocus: HTMLElement | undefined;
  let stayButton: HTMLButtonElement | undefined;

  function closePrompt(returnToTrigger = true): void {
    if (dialog?.open) dialog.close();
    setPendingLeave(undefined);
    props.manualLeave?.clearRequest();
    setSaveFailed(false);
    if (returnToTrigger) queueMicrotask(() => returnFocus?.focus());
  }

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
    const continueManualLeave = props.manualLeave?.requested()
      ? props.manualLeave.continue
      : undefined;
    closePrompt(false);
    if (continueManualLeave !== undefined) continueManualLeave();
    else leave?.retry(true);
  }

  return (
    <Show when={pendingLeave() !== undefined || props.manualLeave?.requested() === true}>
      <dialog
        class="confirmation-dialog"
        aria-labelledby={headingId}
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
          closePrompt();
        }}
      >
        <h2 id={headingId}>{props.copy.heading}</h2>
        <p id={descriptionId}>{props.copy.description}</p>
        <Show when={saveFailed()}>
          <p role="status">{props.copy.saveFailureMessage}</p>
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
            {saving() ? props.copy.savingActionLabel : props.copy.saveActionLabel}
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
