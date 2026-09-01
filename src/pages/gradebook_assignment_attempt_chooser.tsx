// Bounded, server-owned exact Assignment Attempt choice for Instructor Gradebook inspection.

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
  GradebookAssignmentAttemptChooserSession,
  type GradebookAssignmentAttemptChooserState,
} from "./gradebook_assignment_attempt_chooser_model";

interface GradebookAssignmentAttemptChooserProps {
  readonly client: Pick<ApiClient, "getSubmittedAssignmentAttemptChoices">;
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

/** Lets an Instructor explicitly select one immutable submitted Assignment Attempt for inspection. */
export function GradebookAssignmentAttemptChooser(
  props: GradebookAssignmentAttemptChooserProps,
): JSX.Element {
  const [state, setState] = createSignal<GradebookAssignmentAttemptChooserState>({
    kind: "loading",
  });
  const ready = createMemo(
    (): Extract<GradebookAssignmentAttemptChooserState, { readonly kind: "ready" }> | undefined => {
      const current = state();
      return current.kind === "ready" ? current : undefined;
    },
  );
  const session = new GradebookAssignmentAttemptChooserSession(
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
      class="gradebook-assignment-attempt-chooser"
      aria-labelledby="gradebook-assignment-attempt-chooser-heading"
      aria-describedby="gradebook-assignment-attempt-chooser-copy"
      ref={(element) => {
        dialog = element;
      }}
      onCancel={(event) => {
        event.preventDefault();
        props.onDismiss();
      }}
    >
      <p class="eyebrow">Choose submitted work</p>
      <h2 id="gradebook-assignment-attempt-chooser-heading">
        Choose one submitted Assignment Attempt
      </h2>
      <p id="gradebook-assignment-attempt-chooser-copy">
        Select the exact Assignment Attempt for {props.studentLabel} in {props.assignmentTitle}.
        Inspection shows the Student&apos;s submitted response, permitted correctness, and permitted
        score.
      </p>
      <Show when={state().kind === "loading"}>
        <p class="loading-state" aria-live="polite">
          Loading submitted Assignment Attempts...
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="inline-error" role="alert">
          <p>Submitted Assignment Attempts could not load. Try again or return to the Gradebook.</p>
          <button class="quiet-action" type="button" onClick={() => void session.retry()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={ready()}>
        {(loaded) => (
          <>
            <ul
              class="gradebook-assignment-attempt-choice-list"
              aria-label="Submitted Assignment Attempts"
            >
              <For
                each={loaded().rows}
                fallback={<li>No submitted Assignment Attempts are available.</li>}
              >
                {(assignmentAttempt) => (
                  <li>
                    <div>
                      <strong>{formatSubmissionTime(assignmentAttempt.submittedAt)}</strong>
                      <Show when={assignmentAttempt.scoreSelected}>
                        <span class="gradebook-score-selected">Used for the current score</span>
                      </Show>
                    </div>
                    <A
                      class="primary-action"
                      href={inspectedStudentWorkUrl(
                        props.course,
                        props.membership,
                        props.assignment,
                        assignmentAttempt.assignmentAttempt,
                        props.operation,
                      )}
                    >
                      Inspect this submitted Assignment Attempt
                    </A>
                  </li>
                )}
              </For>
            </ul>
            <Show when={loaded().moreError}>
              <div class="inline-error" role="alert">
                <p>
                  More submitted Assignment Attempts could not load. The listed attempts remain
                  available.
                </p>
                <button class="quiet-action" type="button" onClick={() => void session.loadMore()}>
                  Try loading more Assignment Attempts
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
                {loaded().loadingMore
                  ? "Loading more Assignment Attempts..."
                  : "Load more submitted Assignment Attempts"}
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
