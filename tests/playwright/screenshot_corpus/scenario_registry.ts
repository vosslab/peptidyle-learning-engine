// The only executable mapping from manifest scenario IDs to visible browser workflows.

import { INSTRUCTOR_SCENARIOS } from "./scenarios_instructor";
import { INSTRUCTOR_POOL_SCENARIOS } from "./scenarios_instructor_pools";
import { INSTRUCTOR_TEMPLATE_SCENARIOS } from "./scenarios_instructor_templates";
import { INSTRUCTOR_WEBWORK_SCENARIOS } from "./scenarios_instructor_webwork";
import { PUBLIC_SCENARIOS } from "./scenarios_public";
import type { ScenarioDefinition } from "./scenario_types";
import { STUDENT_SCENARIOS } from "./scenarios_student";
import { STUDENT_TYPE_SCENARIOS } from "./scenarios_student_types";
import { SYSADMIN_SCENARIOS } from "./scenarios_sysadmin";

export const SCREENSHOT_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  ...PUBLIC_SCENARIOS,
  ...INSTRUCTOR_SCENARIOS,
  ...INSTRUCTOR_POOL_SCENARIOS,
  ...INSTRUCTOR_TEMPLATE_SCENARIOS,
  ...INSTRUCTOR_WEBWORK_SCENARIOS,
  ...STUDENT_SCENARIOS,
  ...STUDENT_TYPE_SCENARIOS,
  ...SYSADMIN_SCENARIOS,
];
