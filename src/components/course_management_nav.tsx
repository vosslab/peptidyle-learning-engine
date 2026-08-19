// One shared course-local navigation layer for instructor workflows.

import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import type { CourseReference } from "../../generated/api/CourseReference";
import { courseRouteReference } from "../navigation/public_route";

export type CourseManagementSection =
  "assignments" | "newAssignment" | "students" | "gradebook" | "appearance";

interface CourseManagementNavProps {
  readonly courseReference: CourseReference;
  readonly active?: CourseManagementSection;
}

function current(active: boolean): "page" | undefined {
  return active ? "page" : undefined;
}

export function CourseManagementNav(props: CourseManagementNavProps): JSX.Element {
  const reference = courseRouteReference(props.courseReference);
  return (
    <nav class="course-management-nav" aria-label="Course management">
      <A href={`/courses/${reference}`} end aria-current={current(props.active === "assignments")}>
        Assignments
      </A>
      <A
        href={`/instructor/courses/${reference}/assignments/new`}
        aria-current={current(props.active === "newAssignment")}
      >
        New assignment
      </A>
      <A
        href={`/instructor/courses/${reference}/students`}
        aria-current={current(props.active === "students")}
      >
        Students
      </A>
      <A
        href={`/instructor/courses/${reference}/gradebook`}
        aria-current={current(props.active === "gradebook")}
      >
        Gradebook
      </A>
      <A
        href={`/instructor/courses/${reference}/appearance`}
        aria-current={current(props.active === "appearance")}
      >
        Appearance
      </A>
    </nav>
  );
}
