import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import { assignmentWorkspacePath } from "./assignment_workspace_paths";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";

/**
 * The direct C-/A- resource has no grading-operations projection.  Keep this
 * route honest until a separately authorized direct producer is added.
 */
export function AssignmentWorkspaceOperationsPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  return (
    <section
      class="assignment-workspace-operations route-error"
      data-route-surface="assignmentWorkspaceOperations"
      role="alert"
    >
      <p class="eyebrow">Assignment workspace</p>
      <h1>Grading operations unavailable</h1>
      <p>This Assignment's direct workspace does not provide grading-operation data.</p>
      <A
        class="primary-link"
        href={assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference)}
      >
        Return to assignment
      </A>
    </section>
  );
}
