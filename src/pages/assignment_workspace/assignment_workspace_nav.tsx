// assignment_workspace_nav.tsx - four-task navigation for an Instructor assignment.

import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import type { AssignmentRouteReference, CourseRouteReference } from "../../navigation/public_route";
import {
  assignmentWorkspacePath,
  type AssignmentWorkspaceSection,
} from "./assignment_workspace_paths";

export {
  assignmentWorkspacePath,
  type AssignmentWorkspaceSection,
} from "./assignment_workspace_paths";

export interface AssignmentWorkspaceNavProps {
  readonly courseReference: CourseRouteReference;
  readonly assignmentReference: AssignmentRouteReference;
  readonly active: AssignmentWorkspaceSection;
}

function current(active: boolean): "page" | undefined {
  return active ? "page" : undefined;
}

/** Keeps assignment-local tasks separate from the broader course management navigation. */
export function AssignmentWorkspaceNav(props: AssignmentWorkspaceNavProps): JSX.Element {
  return (
    <nav class="assignment-workspace-nav" aria-label="Assignment workspace">
      <A
        href={assignmentWorkspacePath(props.courseReference, props.assignmentReference)}
        end
        aria-current={current(props.active === "overview")}
      >
        Overview
      </A>
      <A
        href={assignmentWorkspacePath(
          props.courseReference,
          props.assignmentReference,
          "questions",
        )}
        aria-current={current(props.active === "questions")}
      >
        Questions
      </A>
      <A
        href={assignmentWorkspacePath(props.courseReference, props.assignmentReference, "policies")}
        aria-current={current(props.active === "policies")}
      >
        Policies
      </A>
      <A
        href={assignmentWorkspacePath(
          props.courseReference,
          props.assignmentReference,
          "studentView",
        )}
        aria-current={current(props.active === "studentView")}
      >
        Student view
      </A>
    </nav>
  );
}
