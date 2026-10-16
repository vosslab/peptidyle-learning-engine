import { useBeforeLeave, type BeforeLeaveEventArgs } from "@solidjs/router";
import {
  Show,
  createEffect,
  createSignal,
  createUniqueId,
  on,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
} from "solid-js";

export interface UnsavedChangesGuardCopy {
  readonly heading: string;
  readonly description: JSX.Element;
  readonly saveActionLabel?: string;
  readonly savingActionLabel?: string;
  readonly saveFailureMessage?: JSX.Element;
}

export interface UnsavedChangesGuardProps {
  readonly dirty: Accessor<boolean>;
  /** Limits content-owned transitions; route navigation always checks every guard. */
  readonly scope?: symbol;
  /** Returns true only after the local edits are safely persisted. */
  readonly save?: () => Promise<boolean>;
  /** The editor's actual pending write, cleared by the editor when it settles. */
  readonly inFlightSave?: Accessor<Promise<boolean> | undefined>;
  readonly copy: UnsavedChangesGuardCopy;
  /** Automatically save on route leave and show the prompt only if that save fails. */
  readonly autoSaveOnLeave?: boolean;
  /** Optional component-owned leave action, such as closing an editor dialog. */
  readonly manualLeave?: {
    readonly requested: Accessor<boolean>;
    readonly clearRequest: () => void;
    readonly continue: () => void;
  };
}

/** Shares an editor's explicit Save promise with its leave guard. */
export function trackEditorSave(save: () => Promise<boolean>): {
  readonly save: () => Promise<boolean>;
  readonly inFlightSave: () => Promise<boolean> | undefined;
} {
  let pending: Promise<boolean> | undefined;
  return {
    save: (): Promise<boolean> => {
      if (pending !== undefined) return pending;
      const operation = save();
      pending = operation;
      void operation.then(
        () => {
          if (pending === operation) pending = undefined;
        },
        () => {
          if (pending === operation) pending = undefined;
        },
      );
      return operation;
    },
    inFlightSave: () => pending,
  };
}

interface RegisteredGuard {
  readonly scope?: symbol;
  readonly needsProtection: () => boolean;
  readonly decideLeave: () => Promise<boolean>;
}

const registeredGuards = new Set<RegisteredGuard>();
let routeDecisionPending = false;
let sharedDecision: Promise<boolean> | undefined;
let sharedDecisionScope: symbol | undefined;

/** Confirms one destructive Library transition using the route guard's editor registry. */
export function confirmUnsavedChanges(scope?: symbol): Promise<boolean> {
  if (sharedDecision !== undefined) {
    if (sharedDecisionScope === scope) return sharedDecision;
    return sharedDecision.then((accepted) => (accepted ? confirmUnsavedChanges(scope) : false));
  }
  sharedDecisionScope = scope;
  sharedDecision = decideRegisteredGuards(scope).finally(() => {
    sharedDecision = undefined;
    sharedDecisionScope = undefined;
  });
  return sharedDecision;
}

async function decideRegisteredGuards(scope?: symbol): Promise<boolean> {
  const approved = new Set<RegisteredGuard>();
  // Discard leaves dirty() true until navigation unmounts the editor. Recheck
  // the registry so another editor dirtied during a decision is included.
  while (true) {
    const next = [...registeredGuards].find(
      (guard) =>
        !approved.has(guard) &&
        (scope === undefined || guard.scope === scope) &&
        guard.needsProtection(),
    );
    if (next === undefined) return true;
    if (!(await next.decideLeave())) return false;
    approved.add(next);
  }
}

async function coordinateRouteLeave(event: BeforeLeaveEventArgs): Promise<void> {
  try {
    if (await confirmUnsavedChanges()) event.retry(true);
  } finally {
    routeDecisionPending = false;
  }
}

/** Protects meaningful editor work during route, browser, and component-owned leave. */
export function UnsavedChangesGuard(props: UnsavedChangesGuardProps): JSX.Element {
  const [promptVisible, setPromptVisible] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [saveFailed, setSaveFailed] = createSignal(false);
  const headingId = `unsaved-changes-heading-${createUniqueId()}`;
  const descriptionId = `unsaved-changes-copy-${createUniqueId()}`;
  let dialog: HTMLDialogElement | undefined;
  let returnFocus: HTMLElement | undefined;
  let stayButton: HTMLButtonElement | undefined;
  let resolvePrompt: ((proceed: boolean) => void) | undefined;
  let decision: Promise<boolean> | undefined;
  let disposed = false;

  function finishPrompt(proceed: boolean): void {
    if (dialog?.open) dialog.close();
    setPromptVisible(false);
    setSaveFailed(false);
    const resolve = resolvePrompt;
    resolvePrompt = undefined;
    resolve?.(proceed);
    if (!proceed && !disposed) queueMicrotask(() => returnFocus?.focus());
  }

  function needsProtection(): boolean {
    return props.dirty() || props.inFlightSave?.() !== undefined;
  }

  async function waitForPendingSave(): Promise<boolean | undefined> {
    let pending = props.inFlightSave?.();
    if (pending === undefined) return undefined;
    let outcome = true;
    setSaving(true);
    try {
      while (pending !== undefined && !disposed) {
        try {
          outcome = await pending;
        } catch {
          outcome = false;
        }
        const next = props.inFlightSave?.();
        if (next === pending) break;
        pending = next;
      }
      return outcome;
    } finally {
      setSaving(false);
    }
  }

  async function saveAndContinue(): Promise<void> {
    if (saving()) return;
    setSaving(true);
    setSaveFailed(false);
    try {
      // A write may have started after the prompt opened. Join it, never write twice.
      const pending = props.inFlightSave?.();
      const saved = pending !== undefined ? await pending : await props.save?.();
      if (saved === true && (disposed || !props.dirty())) finishPrompt(true);
      else if (disposed) finishPrompt(false);
      else setSaveFailed(true);
    } catch {
      if (disposed) finishPrompt(false);
      else setSaveFailed(true);
    } finally {
      setSaving(false);
    }
  }

  function decideLeave(): Promise<boolean> {
    if (decision !== undefined) return decision;
    decision = (async (): Promise<boolean> => {
      returnFocus =
        document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
      const priorSave = await waitForPendingSave();
      // A successful pending write may unmount its editor through onSuccess.
      if (disposed) return priorSave === true;
      if (!props.dirty()) return true;
      setSaveFailed(priorSave === false);
      if (props.autoSaveOnLeave === true && priorSave !== false && props.save !== undefined) {
        // The prompt exists before auto-save, but is shown only on failure.
        const result = new Promise<boolean>((resolve) => {
          resolvePrompt = resolve;
        });
        await saveAndContinue();
        if (resolvePrompt === undefined) return result;
        setPromptVisible(true);
        return result;
      }
      return new Promise<boolean>((resolve) => {
        resolvePrompt = resolve;
        setPromptVisible(true);
      });
    })().finally(() => {
      decision = undefined;
    });
    return decision;
  }

  const guard: RegisteredGuard = { scope: props.scope, needsProtection, decideLeave };
  registeredGuards.add(guard);
  onCleanup(() => {
    disposed = true;
    registeredGuards.delete(guard);
    // A Save-and-continue write can unmount its editor before the await resumes.
    // Let that write settle the existing prompt instead of treating it as Stay.
    if (!saving()) finishPrompt(false);
  });

  useBeforeLeave((event) => {
    if (event.defaultPrevented || ![...registeredGuards].some((item) => item.needsProtection()))
      return;
    event.preventDefault();
    if (routeDecisionPending) return;
    routeDecisionPending = true;
    void coordinateRouteLeave(event);
  });

  createEffect(
    on(
      () => props.manualLeave?.requested(),
      (requested) => {
        if (!requested) return;
        void decideLeave().then((proceed) => {
          if (disposed || !props.manualLeave?.requested()) return;
          props.manualLeave.clearRequest();
          if (proceed) props.manualLeave.continue();
        });
      },
    ),
  );

  onMount(() => {
    const warnBeforeUnload = (event: BeforeUnloadEvent): void => {
      if (!needsProtection()) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warnBeforeUnload);
    onCleanup(() => window.removeEventListener("beforeunload", warnBeforeUnload));
  });

  return (
    <Show when={promptVisible()}>
      <dialog
        class="confirmation-dialog"
        aria-labelledby={headingId}
        aria-describedby={descriptionId}
        ref={(element) => {
          dialog = element;
          queueMicrotask(() => {
            if (!element.isConnected || !promptVisible()) return;
            element.showModal();
            stayButton?.focus();
          });
        }}
        onCancel={(event) => {
          event.preventDefault();
          finishPrompt(false);
        }}
      >
        <h2 id={headingId}>{props.copy.heading}</h2>
        <p id={descriptionId}>{props.copy.description}</p>
        <Show when={saveFailed()}>
          <p role="status">
            {props.copy.saveFailureMessage ?? "Save did not finish. Your edits remain here."}
          </p>
        </Show>
        <div class="action-row">
          <button
            ref={(element) => (stayButton = element)}
            class="quiet-action"
            type="button"
            disabled={saving()}
            onClick={() => finishPrompt(false)}
          >
            Stay and keep editing
          </button>
          <Show when={props.save !== undefined}>
            <button type="button" disabled={saving()} onClick={() => void saveAndContinue()}>
              {saving()
                ? (props.copy.savingActionLabel ?? "Saving...")
                : (props.copy.saveActionLabel ?? "Save and continue")}
            </button>
          </Show>
          <button
            class="primary-action"
            type="button"
            disabled={saving()}
            onClick={() => finishPrompt(true)}
          >
            Discard and continue
          </button>
        </div>
      </dialog>
    </Show>
  );
}
