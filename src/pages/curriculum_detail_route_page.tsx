// curriculum_detail_route_page.tsx - route-owned composition boundary for one reusable curriculum.

import type { JSX } from "solid-js";

import type { ReusableCurriculumClient } from "../api/reusable_curriculum";
import type {
  ProblemPickerSource,
  ProblemPickerSourceRepository,
} from "../features/problem_picker";
import {
  CurriculumDetailWorkspace,
  type CurriculumDetailWorkspaceProps,
} from "../features/reusable_curriculum/reusable_curriculum_workspace";

export type CurriculumDetailRoutePageProps = CurriculumDetailWorkspaceProps;

/** Integrators pass the BP-* route parameter after ordinary route decoding. */
export function CurriculumDetailRoutePage(props: CurriculumDetailRoutePageProps): JSX.Element {
  return <CurriculumDetailWorkspace {...props} />;
}

export type { ReusableCurriculumClient, ProblemPickerSource, ProblemPickerSourceRepository };
