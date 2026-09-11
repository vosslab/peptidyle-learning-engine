import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import { assignmentWorkspacePath } from "./assignment_workspace_paths";

/** Answer-free summary of the one direct Assignment resource. */
export function AssignmentWorkspaceOverviewPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const assignment = (): ReturnType<typeof workspace.assignment>["workspace"] =>
    workspace.assignment().workspace;
  const path = (section?: "questions" | "policies"): string =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference, section);

  return (
    <section class="assignment-workspace-overview" aria-labelledby="assignment-workspace-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-workspace-heading">{assignment().title}</h1>
        <p class="page-lede">Current edit {assignment().editNumber}</p>
      </header>
      <div class="assignment-workspace-grid">
        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-status-heading"
        >
          <h2 id="assignment-status-heading">Current status</h2>
          <dl class="assignment-facts">
            <div>
              <dt>Assignment status</dt>
              <dd>{assignment().status}</dd>
            </div>
            <div>
              <dt>Questions</dt>
              <dd>{assignment().questions.length}</dd>
            </div>
            <div>
              <dt>Due</dt>
              <dd>{assignment().dueAt ?? "No due date"}</dd>
            </div>
            <div>
              <dt>Time zone</dt>
              <dd>{assignment().displayTimeZone}</dd>
            </div>
          </dl>
        </section>
        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-next-heading"
        >
          <h2 id="assignment-next-heading">Edit this assignment</h2>
          <p>
            Questions selects and orders the fixed Questions. Policies controls delivery and
            feedback.
          </p>
          <p class="assignment-workspace-action-row">
            <A class="primary-link" href={path("questions")}>
              Edit Questions
            </A>
            <A class="quiet-link" href={path("policies")}>
              Edit Policies
            </A>
          </p>
        </section>
        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-instructions-heading"
        >
          <h2 id="assignment-instructions-heading">Student instructions</h2>
          <Show
            when={assignment().instructions.length > 0}
            fallback={<p>No Student instructions have been added.</p>}
          >
            <p class="plain-text-instructions">{assignment().instructions}</p>
          </Show>
        </section>
      </div>
    </section>
  );
}
