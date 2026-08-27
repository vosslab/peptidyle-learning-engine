import { A } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import {
  LearnerAssignmentPresentation,
  toLearnerAssignmentPresentationData,
} from "../../components/learner_assignment_presentation";
import { assignmentWorkspacePath } from "./assignment_workspace_nav";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import {
  STUDENT_VIEW_CUE,
  STUDENT_VIEW_ENTRY_PATH,
  studentViewFailureState,
  type StudentViewState,
} from "./assignment_workspace_student_view_model";

function StudentViewFailure(props: {
  readonly kind: "unavailable" | "error";
  readonly returnPath: string;
  readonly retry: () => void;
  readonly registerRetryButton: (element: HTMLButtonElement) => void;
}): JSX.Element {
  const unavailable = props.kind === "unavailable";
  return (
    <section
      class="assignment-workspace-student-view"
      aria-labelledby="student-view-heading"
      role="alert"
    >
      <div class="student-view-cue" role="note">
        {STUDENT_VIEW_CUE}
      </div>
      <div class="student-view-actions">
        <A class="primary-link" href={props.returnPath}>
          Return to assignment
        </A>
      </div>
      <p class="eyebrow">Assignment workspace</p>
      <h1 id="student-view-heading">Student view unavailable</h1>
      <p>
        {unavailable
          ? "The answer-free learner landing is not available for this assignment."
          : "The answer-free learner landing could not load. Try again without leaving this workspace."}
      </p>
      <button
        class="quiet-action"
        type="button"
        onClick={props.retry}
        ref={props.registerRetryButton}
      >
        Retry Student view
      </button>
    </section>
  );
}

export function AssignmentWorkspaceStudentViewPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const [state, setState] = createSignal<StudentViewState>({ kind: "loading" });
  const returnPath = assignmentWorkspacePath(
    workspace.courseReference,
    workspace.assignmentReference,
  );
  let retryButton: HTMLButtonElement | undefined;

  function registerRetryButton(element: HTMLButtonElement): void {
    retryButton = element;
  }

  async function load(): Promise<void> {
    setState({ kind: "loading" });
    try {
      const projection = await workspace.client.getInstructorStudentView(
        workspace.courseId,
        workspace.assignmentId,
      );
      setState({
        kind: "ready",
        assignment: toLearnerAssignmentPresentationData(projection),
      });
    } catch (error: unknown) {
      setState({ kind: studentViewFailureState(error) });
      requestAnimationFrame(() => retryButton?.focus());
    }
  }

  onMount(() => void load());

  const readyState = (): Extract<StudentViewState, { readonly kind: "ready" }> | undefined => {
    const current = state();
    return current.kind === "ready" ? current : undefined;
  };

  return (
    <Show
      when={readyState()}
      keyed
      fallback={
        <Show
          when={state().kind !== "loading"}
          fallback={
            <section
              class="assignment-workspace-student-view"
              aria-labelledby="student-view-heading"
              aria-busy="true"
            >
              <div class="student-view-cue" role="note">
                {STUDENT_VIEW_CUE}
              </div>
              <div class="student-view-actions">
                <A class="primary-link" href={returnPath}>
                  Return to assignment
                </A>
              </div>
              <p class="eyebrow">Assignment workspace</p>
              <h1 id="student-view-heading">Student view</h1>
              <p class="loading-state" role="status" aria-live="polite">
                Loading the current live assignment...
              </p>
            </section>
          }
        >
          <StudentViewFailure
            kind={state().kind === "unavailable" ? "unavailable" : "error"}
            returnPath={returnPath}
            retry={() => void load()}
            registerRetryButton={registerRetryButton}
          />
        </Show>
      }
    >
      {(ready) => (
        <section class="assignment-workspace-student-view" aria-label="Student view">
          <LearnerAssignmentPresentation
            assignment={ready.assignment}
            contextCue={<span>{STUDENT_VIEW_CUE}</span>}
            returnAction={
              <A class="primary-link" href={returnPath}>
                Return to assignment
              </A>
            }
            primaryAction={
              <div class="student-view-entry">
                <p>
                  Graded work belongs to the ordinary Student account and its live assignment run.
                </p>
                <A class="primary-link wide-action" href={STUDENT_VIEW_ENTRY_PATH}>
                  Open live-demo Student entry
                </A>
              </div>
            }
          />
        </section>
      )}
    </Show>
  );
}
