// v2_report.ts - isolated schema-v2 public-report child.

import {
  readV2WalkthroughState,
  renderV2VisibleOutcomeReport,
} from "../../playwright/simulator/v2_visible_outcome_report";
import { childInputsFromArguments } from "./child_inputs";

function main(): void {
  const inputs = childInputsFromArguments(process.argv.slice(2), "learner_journey");
  const state = readV2WalkthroughState(inputs.journeyStateFile);
  const rendered = renderV2VisibleOutcomeReport(inputs.masterSeed, state);
  if (rendered === undefined) throw new Error("invalid fixed report input");
  process.stdout.write(rendered);
}

try {
  main();
} catch {
  process.exitCode = 1;
}
