import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import { assignmentWorkspacePath } from "./assignment_workspace_paths";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";

/** Keeps the route visible without manufacturing a Student projection from Instructor data. */
export function AssignmentWorkspaceStudentViewPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  return (
    <section
      class="assignment-workspace-student-view route-error"
      role="alert"
      aria-labelledby="student-view-heading"
    >
      <p class="eyebrow">Assignment workspace</p>
      <h1 id="student-view-heading">Student view unavailable</h1>
      <p>The direct Assignment workspace has no answer-free Student-view projection.</p>
      <A
        class="primary-link"
        href={assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference)}
      >
        Return to assignment
      </A>
    </section>
  );
}
