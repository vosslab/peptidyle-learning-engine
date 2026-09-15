import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";

/** Keeps the route visible without manufacturing a Student projection from Instructor data. */
export function AssessmentWorkspaceStudentViewPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  return (
    <section
      class="assessment-workspace-student-view route-error"
      role="alert"
      aria-labelledby="student-view-heading"
    >
      <p class="eyebrow">Assessment workspace</p>
      <h1 id="student-view-heading">Student view unavailable</h1>
      <p>The direct Assessment workspace has no answer-free Student-view projection.</p>
      <A
        class="primary-link"
        href={assessmentWorkspacePath(workspace.courseReference, workspace.assessmentReference)}
      >
        Return to assessment
      </A>
    </section>
  );
}
