import { A } from "@solidjs/router";
import { Show, type Accessor, type JSX } from "solid-js";

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseReference } from "../../generated/api/CourseReference";
import { assignmentRouteReference, courseRouteReference } from "../navigation/public_route";

export interface AssignmentEditorSavedLinksProps {
  readonly assignmentReference: Accessor<AssignmentReference | undefined>;
  readonly courseReference: CourseReference;
}

export function AssignmentEditorSavedLinks(props: AssignmentEditorSavedLinksProps): JSX.Element {
  return (
    <Show when={props.assignmentReference()}>
      {(assignmentReference) => (
        <>
          <p>
            <A
              class="quiet-link"
              href={`/courses/${courseRouteReference(props.courseReference)}/assignments/${assignmentRouteReference(assignmentReference())}`}
            >
              View learner-facing assignment overview
            </A>
          </p>
          <p>
            <A
              class="quiet-link"
              href={`/instructor/courses/${courseRouteReference(props.courseReference)}/assignments/${assignmentRouteReference(assignmentReference())}/delivery-check`}
            >
              Check assignment delivery
            </A>
          </p>
        </>
      )}
    </Show>
  );
}
