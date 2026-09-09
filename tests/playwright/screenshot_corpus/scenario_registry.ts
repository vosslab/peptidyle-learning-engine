// The only executable mapping from manifest scenario IDs to visible browser workflows.

import { INSTRUCTOR_SCENARIOS } from "./scenarios_instructor";
import { PUBLIC_SCENARIOS } from "./scenarios_public";
import type { ScenarioDefinition } from "./scenario_types";
import { STUDENT_SCENARIOS } from "./scenarios_student";
import { SYSADMIN_SCENARIOS } from "./scenarios_sysadmin";

export const SCREENSHOT_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  ...PUBLIC_SCENARIOS,
  ...INSTRUCTOR_SCENARIOS,
  ...STUDENT_SCENARIOS,
  ...SYSADMIN_SCENARIOS,
];
