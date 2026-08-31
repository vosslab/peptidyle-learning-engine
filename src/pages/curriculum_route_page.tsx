// curriculum_route_page.tsx - route-owned composition boundary for the Blueprint Course list.

import type { JSX } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import type {
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  CurriculumWorkspace,
  type CurriculumWorkspaceProps,
} from "../features/blueprint_course/blueprint_course_workspace";

export type CurriculumRoutePageProps = CurriculumWorkspaceProps;

/** Integrators supply the browser curriculum capability and current answer-free picker sources. */
export function CurriculumRoutePage(props: CurriculumRoutePageProps): JSX.Element {
  return <CurriculumWorkspace {...props} />;
}

export type { BlueprintCourseClient, QuestionPickerSource, QuestionPickerSourceRepository };
