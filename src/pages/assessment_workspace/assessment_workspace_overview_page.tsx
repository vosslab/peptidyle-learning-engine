import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { PageFrame } from "../../components/page_frame";
import { formatLocalWallClockDateTime } from "../../format_datetime";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";

/** Answer-free summary of the one direct Assessment resource. */
export function AssessmentWorkspaceOverviewPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const assessment = (): ReturnType<typeof workspace.assessment>["workspace"] =>
    workspace.assessment().workspace;
  const path = (section?: "questions" | "policies"): string =>
    assessmentWorkspacePath(workspace.courseInstanceId, workspace.assessmentId, section);

  return (
    <PageFrame
      contentClass="assessment-workspace-overview"
      routeSurface="assessmentWorkspace"
      headingId="assessment-workspace-heading"
      eyebrow="Assessment workspace"
      title={assessment().title}
      lede={`Current edit ${assessment().assessmentEditNumber}`}
    >
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
              <dd>
                <Show when={assessment().dueAt} fallback="No due date">
                  {(dueAt) => formatLocalWallClockDateTime(dueAt())}
                </Show>
              </dd>
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
            The Assessment Question Editor selects, adds, removes, and orders Questions and Question
            Pools. The Assessment Properties Editor controls dates, scoring, attempts, late work,
            and what Students can see.
          </p>
          <p class="assessment-workspace-action-row">
            <A class="primary-link" href={path("questions")}>
              Assessment Question Editor
            </A>
            <A class="quiet-link" href={path("policies")}>
              Assessment Properties Editor
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
    </PageFrame>
  );
}
