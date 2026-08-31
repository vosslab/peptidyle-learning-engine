import { For, Show, createMemo, createSignal, type JSX } from "solid-js";

import type { AccountApprovalView } from "../../../generated/api/AccountApprovalView";
import type { SysadminInstructorCandidateView } from "../../../generated/api/SysadminInstructorCandidateView";
import type { OrdinaryBrowserApiClient } from "../../api/client";
import type { ApplicationApi } from "../../api/application_api";
import { ApiRequestError } from "../../api/http_client/error";
import {
  appendInstructorCandidatePage,
  approvalFailureCopy,
  approvalReloadRequired,
  approvalSuccessCopy,
  candidateAction,
  candidateActionLabel,
  candidateActionRevision,
  candidateApprovalLabel,
  isCandidateQueryEligible,
  type InstructorApprovalAction,
} from "./sysadmin_instructor_approval_model";
import "./teaching_operations_panels.css";

interface SysadminInstructorApprovalPanelProps {
  readonly applicationApi: Pick<ApplicationApi<OrdinaryBrowserApiClient>, "client">;
}

interface PendingApprovalAction {
  readonly action: InstructorApprovalAction;
  readonly candidate: SysadminInstructorCandidateView;
  readonly trigger: HTMLButtonElement;
}

function requestStatus(error: unknown): number | undefined {
  return error instanceof ApiRequestError ? error.status : undefined;
}

/** Sysadmin-only candidate discovery; the server remains the authority for every change. */
export function SysadminInstructorApprovalPanel(
  props: SysadminInstructorApprovalPanelProps,
): JSX.Element {
  const [query, setQuery] = createSignal("");
  const [candidates, setCandidates] = createSignal<ReadonlyArray<SysadminInstructorCandidateView>>(
    [],
  );
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [searching, setSearching] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string>();
  const [announcement, setAnnouncement] = createSignal("");
  const [pendingAction, setPendingAction] = createSignal<PendingApprovalAction>();
  let heading: HTMLHeadingElement | undefined;
  let resultsHeading: HTMLHeadingElement | undefined;
  let statusNode: HTMLParagraphElement | undefined;
  let dialog: HTMLDialogElement | undefined;
  let dialogCancel: HTMLButtonElement | undefined;
  let restoreDialogFocus = true;

  const queryEligible = createMemo(() => isCandidateQueryEligible(query()));

  function focusStatus(): void {
    queueMicrotask(() => statusNode?.focus());
  }

  function focusRefreshedResults(): void {
    queueMicrotask(() => (resultsHeading ?? heading ?? statusNode)?.focus());
  }

  async function searchPage(after: string | null, append: boolean): Promise<boolean> {
    if (!queryEligible()) return false;
    const client = props.applicationApi.client;
    setSearching(true);
    setError(undefined);
    try {
      const page = await client.searchSysadminInstructorCandidates({
        query: query().trim(),
        after,
        size: 20,
      });
      setCandidates((current) =>
        append ? appendInstructorCandidatePage(current, page.candidates) : page.candidates,
      );
      setCursor(page.nextCursor);
      const count = append ? page.candidates.length : page.candidates.length;
      setAnnouncement(
        count === 0
          ? "No matching account is available to approve."
          : `${count} matching account${count === 1 ? "" : "s"} found.`,
      );
      focusStatus();
      return true;
    } catch {
      setError("Matching accounts could not be searched. Check your connection and try again.");
      focusStatus();
      return false;
    } finally {
      setSearching(false);
    }
  }

  async function search(): Promise<void> {
    await searchPage(null, false);
  }

  async function refreshAfterConflict(): Promise<void> {
    const refreshed = await searchPage(null, false);
    if (refreshed) {
      setAnnouncement("The approval state changed. Results were refreshed.");
      focusRefreshedResults();
    }
  }

  function closeDialog(restoreFocus = true): void {
    restoreDialogFocus = restoreFocus;
    if (dialog?.open) dialog.close();
    else {
      const action = pendingAction();
      setPendingAction(undefined);
      if (restoreFocus) queueMicrotask(() => action?.trigger.focus());
    }
  }

  function openDialog(
    action: InstructorApprovalAction,
    candidate: SysadminInstructorCandidateView,
    trigger: HTMLButtonElement,
  ): void {
    setPendingAction({ action, candidate, trigger });
    queueMicrotask(() => {
      if (dialog !== undefined && !dialog.open) dialog.showModal();
      dialogCancel?.focus();
    });
  }

  function updateCandidate(
    original: SysadminInstructorCandidateView,
    approval: AccountApprovalView,
  ): void {
    setCandidates((current) =>
      current.map((candidate) =>
        candidate.account.reference === original.account.reference
          ? { ...candidate, approval }
          : candidate,
      ),
    );
  }

  async function confirmAction(): Promise<void> {
    const pending = pendingAction();
    if (pending === undefined) return;
    const revision = candidateActionRevision(pending.candidate);
    if (pending.action === "revoke" && revision === undefined) {
      setError("The approval state changed. Results were refreshed.");
      closeDialog(false);
      await refreshAfterConflict();
      return;
    }
    setBusy(true);
    setError(undefined);
    try {
      let approval: AccountApprovalView;
      if (pending.action === "approve") {
        approval = await props.applicationApi.client.approveInstructorAccount(
          pending.candidate.account.reference,
          revision,
        );
      } else {
        const revokeRevision = candidateActionRevision(pending.candidate);
        if (revokeRevision === undefined) {
          setError("The approval state changed. Results were refreshed.");
          closeDialog(false);
          await refreshAfterConflict();
          return;
        }
        approval = await props.applicationApi.client.revokeInstructorApproval(
          pending.candidate.account.reference,
          revokeRevision,
        );
      }
      updateCandidate(pending.candidate, approval);
      setAnnouncement(approvalSuccessCopy(pending.candidate.account.display, pending.action));
      closeDialog(false);
      focusStatus();
    } catch (caught: unknown) {
      const status = requestStatus(caught);
      if (approvalReloadRequired(status)) {
        closeDialog(false);
        await refreshAfterConflict();
      } else {
        setError(approvalFailureCopy(status));
        closeDialog(false);
        focusStatus();
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section
      class="teaching-operations-panel sysadmin-instructor-approval"
      aria-labelledby="instructor-approval-heading"
    >
      <h2 ref={(element) => (heading = element)} id="instructor-approval-heading" tabindex={-1}>
        Instructor approval
      </h2>
      <p class="teaching-operations-context">
        Approval makes a person eligible for a later course invitation. It creates no course
        membership.
      </p>
      <p
        class="teaching-operations-status"
        role="status"
        aria-live="polite"
        tabindex={-1}
        ref={(element) => (statusNode = element)}
      >
        {announcement()}
      </p>
      <Show when={error()}>
        {(message) => (
          <section class="inline-error" role="alert">
            <p>{message()}</p>
            <button
              class="quiet-action"
              type="button"
              disabled={searching() || busy()}
              onClick={() => void search()}
            >
              Retry search
            </button>
          </section>
        )}
      </Show>
      <form
        class="teaching-operations-form"
        onSubmit={(event) => {
          event.preventDefault();
          void search();
        }}
      >
        <label for="sysadmin-instructor-candidate-search">
          Find an account by name
          <input
            id="sysadmin-instructor-candidate-search"
            type="search"
            minlength={2}
            maxlength={100}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
          />
        </label>
        <p class="field-help">
          Enter at least two characters. Results contain only available accounts.
        </p>
        <div class="teaching-operations-actions">
          <button type="submit" disabled={!queryEligible() || searching() || busy()}>
            {searching() ? "Searching..." : "Search accounts"}
          </button>
        </div>
      </form>
      <Show when={candidates().length > 0}>
        <section aria-label="Instructor approval search results">
          <h3 ref={(element) => (resultsHeading = element)} tabindex={-1}>
            Search results
          </h3>
          <ul class="teaching-operations-list">
            <For each={candidates()}>
              {(candidate) => {
                const action = candidateAction(candidate);
                return (
                  <li>
                    <div>
                      <strong>{candidate.account.display}</strong>
                      <span>{candidateApprovalLabel(candidate)}</span>
                    </div>
                    <Show when={action}>
                      {(availableAction) => (
                        <button
                          type="button"
                          classList={{
                            "teaching-operations-danger": availableAction() === "revoke",
                          }}
                          disabled={busy() || searching()}
                          onClick={(event) =>
                            openDialog(availableAction(), candidate, event.currentTarget)
                          }
                        >
                          {candidateActionLabel(availableAction())}
                        </button>
                      )}
                    </Show>
                  </li>
                );
              }}
            </For>
          </ul>
        </section>
      </Show>
      <Show when={cursor() !== null}>
        <button
          class="quiet-action"
          type="button"
          disabled={searching() || busy()}
          onClick={() => void searchPage(cursor(), true)}
        >
          {searching() ? "Loading more accounts..." : "Load more accounts"}
        </button>
      </Show>
      <dialog
        class="teaching-operations-confirm"
        aria-labelledby="instructor-approval-confirm-heading"
        ref={(element) => (dialog = element)}
        onCancel={(event) => {
          event.preventDefault();
          closeDialog();
        }}
        onClose={() => {
          const action = pendingAction();
          setPendingAction(undefined);
          if (restoreDialogFocus) queueMicrotask(() => action?.trigger.focus());
          restoreDialogFocus = true;
        }}
      >
        <Show when={pendingAction()}>
          {(pending) => (
            <>
              <h3 id="instructor-approval-confirm-heading">
                {pending().action === "approve"
                  ? "Approve instructor?"
                  : "Revoke instructor approval?"}
              </h3>
              <p>
                {pending().action === "approve"
                  ? `Make ${pending().candidate.account.display} eligible for course invitations? This does not add them to a course.`
                  : `Remove ${pending().candidate.account.display}'s eligibility for future course invitations? Existing course membership is unchanged.`}
              </p>
              <div class="teaching-operations-actions">
                <button type="button" disabled={busy()} onClick={() => void confirmAction()}>
                  {busy() ? "Saving..." : candidateActionLabel(pending().action)}
                </button>
                <button
                  ref={(element) => (dialogCancel = element)}
                  class="quiet-action"
                  type="button"
                  disabled={busy()}
                  onClick={() => closeDialog()}
                >
                  Cancel
                </button>
              </div>
            </>
          )}
        </Show>
      </dialog>
    </section>
  );
}
