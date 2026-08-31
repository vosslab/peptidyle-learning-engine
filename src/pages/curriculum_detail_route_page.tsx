// curriculum_detail_route_page.tsx - route-owned composition boundary for one Blueprint Course.

import type { JSX } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import type {
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  CurriculumDetailWorkspace,
  type CurriculumDetailWorkspaceProps,
} from "../features/blueprint_course/blueprint_course_workspace";

export type CurriculumDetailRoutePageProps = CurriculumDetailWorkspaceProps;

/** Integrators pass the BP-* route parameter after ordinary route decoding. */
export function CurriculumDetailRoutePage(props: CurriculumDetailRoutePageProps): JSX.Element {
  return <CurriculumDetailWorkspace {...props} />;
}

export type { BlueprintCourseClient, QuestionPickerSource, QuestionPickerSourceRepository };
