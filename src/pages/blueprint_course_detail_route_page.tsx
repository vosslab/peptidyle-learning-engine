// blueprint_course_detail_route_page.tsx - route-owned composition boundary for one Blueprint Course.

import type { JSX } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import type {
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  BlueprintCourseDetailWorkspace,
  type BlueprintCourseDetailWorkspaceProps,
} from "../features/blueprint_course/blueprint_course_workspace";

export type BlueprintCourseDetailRoutePageProps = BlueprintCourseDetailWorkspaceProps;

/** Integrators pass the BP-* route parameter after ordinary route decoding. */
export function BlueprintCourseDetailRoutePage(
  props: BlueprintCourseDetailRoutePageProps,
): JSX.Element {
  return <BlueprintCourseDetailWorkspace {...props} />;
}

export type { BlueprintCourseClient, QuestionPickerSource, QuestionPickerSourceRepository };
