// blueprint_course_route_page.tsx - route-owned composition boundary for the Blueprint Course list.

import type { JSX } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import type {
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  BlueprintCoursesWorkspace,
  type BlueprintCoursesWorkspaceProps,
} from "../features/blueprint_course/blueprint_course_workspace";

export type BlueprintCoursesRoutePageProps = BlueprintCoursesWorkspaceProps;

/** Integrators supply the browser Blueprint Course capability and current answer-free picker sources. */
export function BlueprintCoursesRoutePage(props: BlueprintCoursesRoutePageProps): JSX.Element {
  return <BlueprintCoursesWorkspace {...props} />;
}

export type { BlueprintCourseClient, QuestionPickerSource, QuestionPickerSourceRepository };
