// scenario_types.ts - executable scenario registry contract.

import type { ScenarioRegistration } from "./manifest";
import type { ScenarioRuntime } from "./runtime";

export interface ScenarioDefinition extends ScenarioRegistration {
  readonly run: (runtime: ScenarioRuntime) => Promise<void>;
}
