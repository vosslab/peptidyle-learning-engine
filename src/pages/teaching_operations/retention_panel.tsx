import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseReference } from "../../../generated/api/CourseReference";
import type { RetentionDispositionView } from "../../../generated/api/RetentionDispositionView";
import type { RetentionReadView } from "../../../generated/api/RetentionReadView";
import type { ApiRuntime } from "../../api/runtime";
import { ApiRequestError } from "../../api/http_client";
import { retentionStateCopy } from "./retention_state_copy";
import {
  retentionActionAvailability,
  retentionFailureCopy,
  retentionOutcomeCopy,
  retentionReloadRequired,
} from "./retention_panel_model";
import "./teaching_operations_panels.css";

type PanelState = "loading" | "unended" | "ready" | "error";
type RetentionAction = "archive" | "delete";

export interface RetentionPanelProps {
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
  readonly runtime: Pick<ApiRuntime, "client">;
  readonly mayExtendRetention: boolean;
}

function requestStatus(error: unknown): number | undefined {
  return error instanceof ApiRequestError ? error.status : undefined;
}

function additionalDays(value: string): number | undefined {
  if (!/^[1-9][0-9]*$/u.test(value)) return undefined;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : undefined;
}

function actionTitle(action: RetentionAction): string {
  return action === "archive" ? "Archive student records" : "Delete student records";
}

/** Server state and outcomes remain authoritative; this panel only presents the available actions. */
export function RetentionPanel(props: RetentionPanelProps): JSX.Element {
  const [state, setState] = createSignal<PanelState>("loading");
  const [retention, setRetention] = createSignal<RetentionReadView>();
  const [message, setMessage] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [pending, setPending] = createSignal<RetentionAction>();
  const [confirmation, setConfirmation] = createSignal("");
  const [confirmationMessage, setConfirmationMessage] = createSignal("");
  const [definitionDisposition, setDefinitionDisposition] =
    createSignal<RetentionDispositionView>("retain");
  const [extensionDays, setExtensionDays] = createSignal("30");
  const [reloadRequired, setReloadRequired] = createSignal(false);
  let statusNode: HTMLParagraphElement | undefined;
  let confirmationDialog: HTMLDialogElement | undefined;
  let confirmationCancel: HTMLButtonElement | undefined;
  let confirmationError: HTMLParagraphElement | undefined;
  let actionOpener: HTMLButtonElement | undefined;
  let restoreConfirmationFocus = true;

  function focusStatus(): void {
    queueMicrotask(() => statusNode?.focus());
  }
  function restoreActionFocus(): void {
    queueMicrotask(() => {
      if (actionOpener?.isConnected) actionOpener.focus();
      else statusNode?.focus();
    });
  }
  function clearConfirmationDialog(restoreFocus: boolean): void {
    setPending(undefined);
    setConfirmation("");
    setConfirmationMessage("");
    if (restoreFocus) restoreActionFocus();
  }
  function closeConfirmationDialog(restoreFocus = true): void {
    if (confirmationDialog?.open) {
      restoreConfirmationFocus = restoreFocus;
      confirmationDialog.close();
    } else clearConfirmationDialog(restoreFocus);
  }
  function openConfirmationDialog(action: RetentionAction, opener: HTMLButtonElement): void {
    actionOpener = opener;
    setConfirmation("");
    setConfirmationMessage("");
    setPending(action);
    queueMicrotask(() => {
      if (confirmationDialog !== undefined && !confirmationDialog.open)
        confirmationDialog.showModal();
      confirmationCancel?.focus();
    });
  }
  function focusConfirmationError(): void {
    queueMicrotask(() => (confirmationError ?? confirmationCancel)?.focus());
  }
  async function load(): Promise<void> {
    setState("loading");
    try {
      const current = await props.runtime.client.getCourseRetention(props.courseId);
      setRetention(current);
      setMessage("Retention state loaded.");
      setState("ready");
      setReloadRequired(false);
    } catch (error: unknown) {
      if (requestStatus(error) === 404) {
        setState("unended");
        setMessage("This course has not entered retention yet.");
      } else {
        setState("error");
        setMessage(retentionFailureCopy(requestStatus(error)));
      }
    }
  }
  async function reloadLatestRetention(): Promise<void> {
    await load();
    focusStatus();
  }
  function handleRetentionFailure(error: unknown, inDialog = false): void {
    const status = requestStatus(error);
    const failure = retentionFailureCopy(status);
    setMessage(failure);
    if (retentionReloadRequired(status)) {
      setReloadRequired(true);
      if (inDialog) {
        closeConfirmationDialog(false);
        queueMicrotask(() => focusStatus());
        return;
      }
    }
    if (inDialog) {
      setConfirmationMessage(failure);
      focusConfirmationError();
      return;
    }
    focusStatus();
  }
  async function endRetention(): Promise<void> {
    setBusy(true);
    try {
      const current = await props.runtime.client.endCourseRetention(props.courseId);
      setRetention(current);
      setMessage("The course retention period has ended.");
      setState("ready");
    } catch (error: unknown) {
      handleRetentionFailure(error);
    } finally {
      setBusy(false);
    }
  }
  async function runAction(): Promise<void> {
    const action = pending();
    const current = retention();
    if (action === undefined || current === undefined) return;
    setBusy(true);
    try {
      const response =
        action === "archive"
          ? await props.runtime.client.archiveCourseRetention(
              props.courseId,
              { assignmentDefinitions: definitionDisposition() },
              current.revision,
            )
          : await props.runtime.client.deleteCourseRetention(props.courseId, current.revision);
      setRetention({
        state: response.state,
        assignmentDefinitions: response.assignmentDefinitions,
        revision: response.revision,
      });
      setMessage(retentionOutcomeCopy(response.outcome));
      closeConfirmationDialog(false);
      queueMicrotask(() => focusStatus());
    } catch (error: unknown) {
      handleRetentionFailure(error, true);
    } finally {
      setBusy(false);
    }
  }
  async function extendRetention(): Promise<void> {
    const days = additionalDays(extensionDays());
    const current = retention();
    if (days === undefined || current === undefined) {
      setMessage("Enter a whole number of additional days.");
      focusStatus();
      return;
    }
    setBusy(true);
    try {
      const updated = await props.runtime.client.extendCourseRetention(
        props.courseId,
        { additionalDays: days },
        current.revision,
      );
      setRetention(updated);
      setMessage("The retention period has been extended.");
    } catch (error: unknown) {
      handleRetentionFailure(error);
    } finally {
      setBusy(false);
    }
  }
  onMount(() => {
    void load();
  });
  return (
    <section class="teaching-operations-panel" aria-labelledby="retention-heading">
      <h2 id="retention-heading">Record retention</h2>
      <p class="teaching-operations-context">
        Server-owned retention actions protect student records.
      </p>
      <p
        class="teaching-operations-status"
        role="status"
        aria-live="polite"
        tabindex="-1"
        ref={(element) => {
          statusNode = element;
        }}
      >
        {message()}
      </p>
      <Show when={state() === "loading"}>
        <p>Loading retention state...</p>
      </Show>
      <Show when={state() === "error"}>
        <button type="button" onClick={() => void load()}>
          Retry loading retention
        </button>
      </Show>
      <Show when={reloadRequired()}>
        <button type="button" disabled={busy()} onClick={() => void reloadLatestRetention()}>
          Reload latest retention state
        </button>
      </Show>
      <Show when={state() === "unended"}>
        <p>End this course before choosing a record-retention action.</p>
        <button type="button" disabled={busy()} onClick={() => void endRetention()}>
          End course and begin retention
        </button>
      </Show>
      <Show when={retention()} keyed>
        {(current) => {
          const availability = retentionActionAvailability(current.state);
          return (
            <div class="teaching-operations-retention">
              <p>{retentionStateCopy(current.state)}</p>
              <p>
                Assignment definitions:{" "}
                {current.assignmentDefinitions === "retain" ? "retained" : "deleted"}.
              </p>
              <Show
                when={current.notification}
                fallback={
                  <p class="teaching-operations-notification">
                    No retention notification is available.
                  </p>
                }
              >
                {(notification) => (
                  <p class="teaching-operations-notification">{notification().copy}</p>
                )}
              </Show>
              <div class="teaching-operations-actions">
                <Show when={availability.archive}>
                  <button
                    type="button"
                    disabled={busy()}
                    onClick={(event) => openConfirmationDialog("archive", event.currentTarget)}
                  >
                    Archive student records
                  </button>
                </Show>
                <Show when={availability.delete}>
                  <button
                    type="button"
                    class="teaching-operations-danger"
                    disabled={busy()}
                    onClick={(event) => openConfirmationDialog("delete", event.currentTarget)}
                  >
                    Delete student records
                  </button>
                </Show>
              </div>
              <Show when={props.mayExtendRetention && availability.extend}>
                <fieldset>
                  <legend>Sysadmin extension</legend>
                  <label>
                    Additional days
                    <input
                      inputmode="numeric"
                      value={extensionDays()}
                      disabled={busy()}
                      onInput={(event) => setExtensionDays(event.currentTarget.value)}
                    />
                  </label>
                  <button type="button" disabled={busy()} onClick={() => void extendRetention()}>
                    Extend retention
                  </button>
                </fieldset>
              </Show>
            </div>
          );
        }}
      </Show>
      <Show when={pending()} keyed>
        {(action) => (
          <dialog
            class="teaching-operations-confirm"
            aria-labelledby="retention-confirm-heading"
            ref={(element) => {
              confirmationDialog = element;
            }}
            onCancel={(event) => {
              event.preventDefault();
              closeConfirmationDialog();
            }}
            onClose={() => {
              clearConfirmationDialog(restoreConfirmationFocus);
              restoreConfirmationFocus = true;
            }}
          >
            <h3 id="retention-confirm-heading">{actionTitle(action)}?</h3>
            <p>
              {action === "delete"
                ? "This permanently deletes student records. Type DELETE to confirm."
                : "This archives Student records from ordinary Student access. Type ARCHIVE to confirm."}
            </p>
            <p
              class="teaching-operations-dialog-status"
              role="alert"
              tabindex="-1"
              ref={(element) => {
                confirmationError = element;
              }}
            >
              {confirmationMessage()}
            </p>
            <Show when={action === "archive"}>
              <label>
                Assignment definitions
                <select
                  value={definitionDisposition()}
                  onChange={(event) =>
                    setDefinitionDisposition(
                      event.currentTarget.value === "delete" ? "delete" : "retain",
                    )
                  }
                >
                  <option value="retain">Retain assignment definitions</option>
                  <option value="delete">Delete assignment definitions</option>
                </select>
              </label>
            </Show>
            <label>
              Confirmation
              <input
                value={confirmation()}
                onInput={(event) => setConfirmation(event.currentTarget.value)}
              />
            </label>
            <div class="teaching-operations-actions">
              <button
                type="button"
                class="teaching-operations-danger"
                disabled={busy() || confirmation() !== (action === "delete" ? "DELETE" : "ARCHIVE")}
                onClick={() => void runAction()}
              >
                Confirm {actionTitle(action).toLowerCase()}
              </button>
              <button
                type="button"
                disabled={busy()}
                ref={(element) => {
                  confirmationCancel = element;
                }}
                onClick={() => {
                  closeConfirmationDialog();
                }}
              >
                Cancel
              </button>
            </div>
          </dialog>
        )}
      </Show>
    </section>
  );
}
