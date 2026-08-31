// Bounded, server-owned exact-run choice for Instructor Gradebook inspection.

import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { InstructorGradingOperationReference } from "../../generated/api/InstructorGradingOperationReference";
import type { ApiClient } from "../api/client";
import { inspectedStudentWorkUrl } from "./gradebook_navigation";
import {
  GradebookRunChooserSession,
  type GradebookRunChooserState,
} from "./gradebook_run_chooser_model";

interface GradebookRunChooserProps {
  readonly client: Pick<ApiClient, "getSubmittedRunChoices">;
  readonly courseId: CourseId;
  readonly course: CourseInstanceReference;
  readonly membership: CourseMembershipReference;
  readonly assignment: AssignmentReference;
  readonly operation?: InstructorGradingOperationReference;
  readonly studentLabel: string;
  readonly assignmentTitle: string;
  readonly onDismiss: () => void;
}

function formatSubmissionTime(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}

/** Lets an Instructor explicitly select one immutable submitted run before inspection. */
export function GradebookRunChooser(props: GradebookRunChooserProps): JSX.Element {
  const [state, setState] = createSignal<GradebookRunChooserState>({ kind: "loading" });
  const ready = createMemo(
    (): Extract<GradebookRunChooserState, { readonly kind: "ready" }> | undefined => {
      const current = state();
      return current.kind === "ready" ? current : undefined;
    },
  );
  const session = new GradebookRunChooserSession(
    {
      courseId: props.courseId,
      membership: props.membership,
      assignment: props.assignment,
      operation: props.operation,
    },
    props.client,
    setState,
  );
  let dialog: HTMLDialogElement | undefined;

  onCleanup(() => {
    session.dispose();
    if (dialog?.open) dialog.close();
  });
  onMount(() => {
    dialog?.showModal();
    void session.start();
  });

  return (
    <dialog
      class="gradebook-run-chooser"
      aria-labelledby="gradebook-run-chooser-heading"
      aria-describedby="gradebook-run-chooser-copy"
      ref={(element) => {
        dialog = element;
      }}
      onCancel={(event) => {
        event.preventDefault();
        props.onDismiss();
      }}
    >
      <p class="eyebrow">Choose submitted work</p>
      <h2 id="gradebook-run-chooser-heading">Choose one submitted run</h2>
      <p id="gradebook-run-chooser-copy">
        Select the exact run for {props.studentLabel} in {props.assignmentTitle}. Inspection shows
        the Student&apos;s submitted response and solution-free grading evidence.
      </p>
      <Show when={state().kind === "loading"}>
        <p class="loading-state" aria-live="polite">
          Loading submitted runs...
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="inline-error" role="alert">
          <p>Submitted runs could not load. Try again or return to the Gradebook.</p>
          <button class="quiet-action" type="button" onClick={() => void session.retry()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={ready()}>
        {(loaded) => (
          <>
            <ul class="gradebook-run-choice-list" aria-label="Submitted runs">
              <For each={loaded().rows} fallback={<li>No submitted runs are available.</li>}>
                {(run) => (
                  <li>
                    <div>
                      <strong>{formatSubmissionTime(run.submittedAt)}</strong>
                      <Show when={run.scoreSelected}>
                        <span class="gradebook-score-selected">Used for the current score</span>
                      </Show>
                    </div>
                    <A
                      class="primary-action"
                      href={inspectedStudentWorkUrl(
                        props.course,
                        props.membership,
                        props.assignment,
                        run.run,
                        props.operation,
                      )}
                    >
                      Inspect this submitted run
                    </A>
                  </li>
                )}
              </For>
            </ul>
            <Show when={loaded().moreError}>
              <div class="inline-error" role="alert">
                <p>More submitted runs could not load. The listed runs remain available.</p>
                <button class="quiet-action" type="button" onClick={() => void session.loadMore()}>
                  Try loading more runs
                </button>
              </div>
            </Show>
            <Show when={loaded().nextCursor !== null && !loaded().moreError}>
              <button
                class="quiet-action"
                type="button"
                disabled={loaded().loadingMore}
                onClick={() => void session.loadMore()}
              >
                {loaded().loadingMore ? "Loading more runs..." : "Load more submitted runs"}
              </button>
            </Show>
          </>
        )}
      </Show>
      <div class="action-row">
        <button class="quiet-action" type="button" onClick={props.onDismiss}>
          Return to Gradebook
        </button>
      </div>
    </dialog>
  );
}
