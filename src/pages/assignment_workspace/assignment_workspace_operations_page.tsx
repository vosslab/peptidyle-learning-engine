// assignment_workspace_operations_page.tsx - visible Instructor recovery for answer-free grading metadata.

import { For, Match, Show, Switch, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type {
  GradingOperationActionId,
  GradingOperationGroupBy,
  InstructorGradingOperationRow,
} from "../../api/decoders/grading_operations";
import {
  gradingOperationsActionFailure,
  gradingOperationsAffectedLearnersLabel,
  gradingOperationsGroupLabel,
  gradingOperationsPositionForGroup,
  gradingOperationsReasonLabel,
  gradingOperationsRetryLabel,
  gradingOperationsStateLabel,
  gradingOperationsTrustGenerationLabel,
  recalculationIntent,
  retryGradingOperationsAction,
  retryOperationIntent,
  type GradingOperationsActionIntent,
} from "./assignment_workspace_operations_model";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import "./assignment_workspace_operations.css";

type ListState = "loading" | "ready" | "error";
type ActionFeedback =
  | { readonly kind: "pending"; readonly message: string }
  | { readonly kind: "retryable"; readonly message: string }
  | { readonly kind: "stale"; readonly message: string }
  | { readonly kind: "success"; readonly message: string };

function initialActionMessage(intent: GradingOperationsActionIntent): string {
  return intent.kind === "retry"
    ? "Requesting another automatic grading attempt."
    : "Requesting an updated grade calculation for this assignment.";
}

function acceptedActionMessage(intent: GradingOperationsActionIntent): string {
  return intent.kind === "retry"
    ? "The grading retry was accepted. The operations list is refreshing."
    : "The assignment recalculation was accepted. The operations list is refreshing.";
}

/** Presents one assignment's recovery work without exposing learner submissions or grading payloads. */
export function AssignmentWorkspaceOperationsPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const [groupBy, setGroupBy] = createSignal<GradingOperationGroupBy>("question");
  const [rows, setRows] = createSignal<ReadonlyArray<InstructorGradingOperationRow>>([]);
  const [cursor, setCursor] = createSignal<string>();
  const [listState, setListState] = createSignal<ListState>("loading");
  const [actionIntent, setActionIntent] = createSignal<GradingOperationsActionIntent>();
  const [feedback, setFeedback] = createSignal<ActionFeedback>();
  const [reloadRequired, setReloadRequired] = createSignal(false);
  let listRequest = 0;
  let statusTarget: HTMLElement | undefined;
  let retryListButton: HTMLButtonElement | undefined;
  let retryActionButton: HTMLButtonElement | undefined;
  let reloadButton: HTMLButtonElement | undefined;

  const hasMore = createMemo(() => cursor() !== undefined);
  const actionPending = createMemo(() => feedback()?.kind === "pending");
  const actionRetryable = createMemo(() => feedback()?.kind === "retryable");
  const actionStale = createMemo(() => feedback()?.kind === "stale");

  function focusStatus(): void {
    requestAnimationFrame(() => statusTarget?.focus());
  }

  function registerStatusTarget(element: HTMLElement): void {
    statusTarget = element;
  }

  function listFailureMessage(): string {
    return "Grading operations could not load. Try loading the operations list again.";
  }

  async function load(
    position = gradingOperationsPositionForGroup(groupBy()),
    append = false,
  ): Promise<void> {
    const request = ++listRequest;
    setListState("loading");
    try {
      const page = await workspace.client.listInstructorGradingOperations(
        workspace.courseId,
        workspace.assignmentId,
        position.groupBy,
        position.cursor,
      );
      if (request !== listRequest) return;
      setRows((current) => (append ? [...current, ...page.items] : page.items));
      setCursor(page.nextCursor ?? undefined);
      setListState("ready");
    } catch (_error: unknown) {
      if (request !== listRequest) return;
      setListState("error");
      requestAnimationFrame(() => retryListButton?.focus());
    }
  }

  function changeGrouping(nextGroupBy: GradingOperationGroupBy): void {
    if (nextGroupBy === groupBy()) return;
    setGroupBy(nextGroupBy);
    setRows([]);
    setCursor(undefined);
    void load(gradingOperationsPositionForGroup(nextGroupBy));
  }

  function loadMore(): void {
    const nextCursor = cursor();
    if (nextCursor === undefined || listState() === "loading") return;
    void load({ groupBy: groupBy(), cursor: nextCursor }, true);
  }

  async function executeAction(intent: GradingOperationsActionIntent): Promise<void> {
    if (actionPending() || reloadRequired()) return;
    setActionIntent(intent);
    setFeedback({ kind: "pending", message: initialActionMessage(intent) });
    try {
      if (intent.kind === "retry") {
        await workspace.client.retryInstructorGradingOperation(
          workspace.courseId,
          workspace.assignmentId,
          intent.operation,
          intent.expectedRevision,
          intent.idempotencyKey,
        );
      } else {
        await workspace.client.recalculateInstructorAssignment(
          workspace.courseId,
          workspace.assignmentId,
          intent.expectedRevision,
          intent.idempotencyKey,
        );
      }
      setActionIntent(undefined);
      setFeedback({ kind: "success", message: acceptedActionMessage(intent) });
      focusStatus();
      await load(gradingOperationsPositionForGroup(groupBy()));
    } catch (error: unknown) {
      const failure = gradingOperationsActionFailure(error);
      setFeedback(failure);
      if (failure.kind === "stale") {
        setReloadRequired(true);
        requestAnimationFrame(() => reloadButton?.focus());
      } else {
        requestAnimationFrame(() => retryActionButton?.focus());
      }
    }
  }

  function startRetry(row: InstructorGradingOperationRow): void {
    const idempotencyKey: GradingOperationActionId = globalThis.crypto.randomUUID();
    void executeAction(
      retryOperationIntent(row.operation.reference, row.operation.revision, idempotencyKey),
    );
  }

  function startRecalculation(): void {
    const idempotencyKey: GradingOperationActionId = globalThis.crypto.randomUUID();
    void executeAction(recalculationIntent(workspace.assignment().revision, idempotencyKey));
  }

  function retryAction(): void {
    const intent = actionIntent();
    if (intent !== undefined) void executeAction(retryGradingOperationsAction(intent));
  }

  async function reloadLatestAssignment(): Promise<void> {
    if (actionPending()) return;
    setFeedback({
      kind: "pending",
      message: "Loading the latest assignment before another grading request.",
    });
    try {
      await workspace.reloadAssignment();
      setActionIntent(undefined);
      setReloadRequired(false);
      setFeedback({
        kind: "success",
        message:
          "Latest assignment loaded. Review the current grading operations before requesting another action.",
      });
      focusStatus();
      await load(gradingOperationsPositionForGroup(groupBy()));
    } catch (_error: unknown) {
      setFeedback({
        kind: "stale",
        message:
          "The latest assignment could not load. Use Reload latest assignment again before continuing.",
      });
      requestAnimationFrame(() => reloadButton?.focus());
    }
  }

  onMount(() => void load());

  return (
    <section
      class="assignment-workspace-operations"
      data-route-surface="assignmentWorkspaceOperations"
      aria-labelledby="grading-operations-heading"
    >
      <header class="assignment-workspace-operations-header">
        <p class="eyebrow">Assignment review</p>
        <h1 id="grading-operations-heading">Grading operations</h1>
        <p>
          Resolve automatic grading interruptions and refresh this assignment's grades.
        </p>
      </header>

      <section class="assignment-workspace-recalculation" aria-labelledby="recalculate-heading">
        <div>
          <h2 id="recalculate-heading">Refresh assignment grades</h2>
          <p>
            Current displayed grades may be temporarily unavailable until the system completes and
            publishes the updated calculation.
          </p>
        </div>
        <button
          class="primary-action"
          type="button"
          disabled={actionPending() || reloadRequired()}
          onClick={startRecalculation}
        >
          Recalculate assignment
        </button>
      </section>

      <section
        class="assignment-workspace-operations-controls"
        aria-label="Grading operation grouping"
      >
        <p>Show operations by:</p>
        <div class="assignment-workspace-operations-group-actions">
          <button
            class="quiet-action"
            type="button"
            aria-pressed={groupBy() === "question"}
            disabled={listState() === "loading" && rows().length === 0}
            onClick={() => changeGrouping("question")}
          >
            Group by question
          </button>
          <button
            class="quiet-action"
            type="button"
            aria-pressed={groupBy() === "learner"}
            disabled={listState() === "loading" && rows().length === 0}
            onClick={() => changeGrouping("learner")}
          >
            Group by learner
          </button>
        </div>
      </section>

      <Show when={feedback()}>
        {(currentFeedback) => (
          <section
            class="assignment-workspace-operations-status"
            classList={{
              "assignment-workspace-operations-status--error":
                currentFeedback().kind === "retryable" || currentFeedback().kind === "stale",
              "assignment-workspace-operations-status--success":
                currentFeedback().kind === "success",
            }}
            role={
              currentFeedback().kind === "retryable" || currentFeedback().kind === "stale"
                ? "alert"
                : "status"
            }
            aria-live={currentFeedback().kind === "pending" ? "polite" : "assertive"}
            tabindex="-1"
            ref={registerStatusTarget}
          >
            <p>{currentFeedback().message}</p>
            <Show when={actionRetryable()}>
              <button
                class="quiet-action"
                type="button"
                onClick={retryAction}
                ref={(element) => (retryActionButton = element)}
              >
                Try the same grading request again
              </button>
            </Show>
            <Show when={actionStale()}>
              <button
                class="primary-action"
                type="button"
                onClick={() => void reloadLatestAssignment()}
                ref={(element) => (reloadButton = element)}
              >
                Reload latest assignment
              </button>
            </Show>
          </section>
        )}
      </Show>

      <Switch>
        <Match when={listState() === "loading" && rows().length === 0}>
          <section class="assignment-workspace-operations-list-state" aria-busy="true">
            <p class="loading-state" role="status" aria-live="polite">
              Loading grading operations...
            </p>
          </section>
        </Match>
        <Match when={listState() === "error" && rows().length === 0}>
          <section class="assignment-workspace-operations-list-state" role="alert">
            <h2>Grading operations could not load</h2>
            <p>{listFailureMessage()}</p>
            <button
              class="primary-action"
              type="button"
              onClick={() => void load()}
              ref={(element) => (retryListButton = element)}
            >
              Retry loading grading operations
            </button>
          </section>
        </Match>
        <Match when={listState() === "ready" && rows().length === 0}>
          <section class="assignment-workspace-operations-list-state" role="status">
            <h2>No grading operations need attention</h2>
            <p>Automatic grading has no current instructor recovery work for this assignment.</p>
          </section>
        </Match>
        <Match when={rows().length > 0}>
          <section class="assignment-workspace-operations-list" aria-label="Grading operations">
            <Show when={listState() === "loading"}>
              <p class="assignment-workspace-operations-refresh" role="status" aria-live="polite">
                Loading additional grading operations...
              </p>
            </Show>
            <Show when={listState() === "error"}>
              <div class="assignment-workspace-operations-inline-error" role="alert">
                <p>{listFailureMessage()}</p>
                <button
                  class="quiet-action"
                  type="button"
                  onClick={() => void load()}
                  ref={(element) => (retryListButton = element)}
                >
                  Retry loading grading operations
                </button>
              </div>
            </Show>
            <For each={rows()}>
              {(row) => (
                <article class="assignment-workspace-operation-row">
                  <div class="assignment-workspace-operation-row-heading">
                    <h2>{gradingOperationsGroupLabel(row)}</h2>
                    <p>{gradingOperationsStateLabel(row)}</p>
                  </div>
                  <p class="assignment-workspace-operation-reason">
                    {gradingOperationsReasonLabel(row)}
                  </p>
                  <dl class="assignment-workspace-operation-facts">
                    <div>
                      <dt>Affected scope</dt>
                      <dd>{gradingOperationsAffectedLearnersLabel(row.affectedLearnerCount)}</dd>
                    </div>
                    <div>
                      <dt>Grading generation</dt>
                      <dd>{gradingOperationsTrustGenerationLabel(row)}</dd>
                    </div>
                  </dl>
                  <Show when={row.operation.nextAction === "retry"}>
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={actionPending() || reloadRequired()}
                      onClick={() => startRetry(row)}
                    >
                      {gradingOperationsRetryLabel(row)}
                    </button>
                  </Show>
                </article>
              )}
            </For>
            <Show when={hasMore() && listState() !== "loading"}>
              <button class="quiet-action" type="button" onClick={loadMore}>
                Load more grading operations
              </button>
            </Show>
          </section>
        </Match>
      </Switch>
    </section>
  );
}
