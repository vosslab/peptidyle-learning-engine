// curriculum_route_page.tsx - route-owned composition boundary for the reusable curriculum list.

import type { JSX } from "solid-js";

import type { ReusableCurriculumClient } from "../api/reusable_curriculum";
import type {
  ProblemPickerSource,
  ProblemPickerSourceRepository,
} from "../features/problem_picker";
import {
  CurriculumWorkspace,
  type CurriculumWorkspaceProps,
} from "../features/reusable_curriculum/reusable_curriculum_workspace";

export type CurriculumRoutePageProps = CurriculumWorkspaceProps;

/** Integrators supply the browser curriculum capability and current answer-free picker sources. */
export function CurriculumRoutePage(props: CurriculumRoutePageProps): JSX.Element {
  return <CurriculumWorkspace {...props} />;
}

export type { ReusableCurriculumClient, ProblemPickerSource, ProblemPickerSourceRepository };
