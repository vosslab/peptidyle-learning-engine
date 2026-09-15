import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";

/** Answer-free summary of the one direct Assessment resource. */
export function AssessmentWorkspaceOverviewPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const assessment = (): ReturnType<typeof workspace.assessment>["workspace"] =>
    workspace.assessment().workspace;
  const path = (section?: "questions" | "policies"): string =>
    assessmentWorkspacePath(workspace.courseReference, workspace.assessmentReference, section);

  return (
    <section class="assessment-workspace-overview" aria-labelledby="assessment-workspace-heading">
      <header class="assessment-workspace-header">
        <p class="eyebrow">Assessment workspace</p>
        <h1 id="assessment-workspace-heading">{assessment().title}</h1>
        <p class="page-lede">Current edit {assessment().editNumber}</p>
      </header>
      <div class="assessment-workspace-grid">
        <section
          class="course-card assessment-workspace-card"
          aria-labelledby="assessment-status-heading"
        >
          <h2 id="assessment-status-heading">Current status</h2>
          <dl class="assessment-facts">
            <div>
              <dt>Assessment status</dt>
              <dd>{assessment().status}</dd>
            </div>
            <div>
              <dt>Questions</dt>
              <dd>{assessment().questions.length}</dd>
            </div>
            <div>
              <dt>Due</dt>
              <dd>{assessment().dueAt ?? "No due date"}</dd>
            </div>
            <div>
              <dt>Time zone</dt>
              <dd>{assessment().displayTimeZone}</dd>
            </div>
          </dl>
        </section>
        <section
          class="course-card assessment-workspace-card"
          aria-labelledby="assessment-next-heading"
        >
          <h2 id="assessment-next-heading">Edit this assessment</h2>
          <p>
            Questions selects and orders the fixed Questions. Policies controls delivery and
            feedback.
          </p>
          <p class="assessment-workspace-action-row">
            <A class="primary-link" href={path("questions")}>
              Edit Questions
            </A>
            <A class="quiet-link" href={path("policies")}>
              Edit Policies
            </A>
          </p>
        </section>
        <section
          class="course-card assessment-workspace-card"
          aria-labelledby="assessment-instructions-heading"
        >
          <h2 id="assessment-instructions-heading">Student instructions</h2>
          <Show
            when={assessment().instructions.length > 0}
            fallback={<p>No Student instructions have been added.</p>}
          >
            <p class="plain-text-instructions">{assessment().instructions}</p>
          </Show>
        </section>
      </div>
    </section>
  );
}
