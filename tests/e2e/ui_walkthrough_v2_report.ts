// ui_walkthrough_v2_report.ts - isolated schema-v2 public-report child.

import {
  readV2WalkthroughState,
  renderV2VisibleOutcomeReport,
} from "../playwright/simulator/v2_visible_outcome_report";

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined || value === "") throw new Error("missing fixed report input");
  return value;
}

function main(): void {
  const state = readV2WalkthroughState(required("PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"));
  const masterSeed = Number(required("PLE_UI_WALKTHROUGH_MASTER_SEED"));
  const rendered = renderV2VisibleOutcomeReport(masterSeed, state);
  if (rendered === undefined) throw new Error("invalid fixed report input");
  process.stdout.write(rendered);
}

try {
  main();
} catch {
  process.exitCode = 1;
}
