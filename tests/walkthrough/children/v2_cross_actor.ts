// v2_cross_actor.ts - isolated descriptor-safe J8 state child.

import { appendV2J8State } from "../../playwright/simulator/v2_j5_j8_state";
import { childInputsFromArguments } from "./child_inputs";

function main(): void {
  const inputs = childInputsFromArguments(process.argv.slice(2), "learner_journey");
  appendV2J8State(inputs.journeyStateFile, 0);
}

try {
  main();
} catch {
  process.exitCode = 1;
}
