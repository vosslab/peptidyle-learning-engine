// ui_walkthrough_v2_cross_actor.ts - isolated descriptor-safe J8 state child.

import { appendV2J8State } from "../playwright/simulator/v2_j5_j8_state";

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined || value === "") throw new Error("missing fixed J8 input");
  return value;
}

function elapsedMs(): number {
  const text = required("PLE_UI_WALKTHROUGH_J8_ELAPSED_MS");
  if (!/^[0-9]+$/u.test(text)) throw new Error("invalid fixed J8 input");
  const value = Number(text);
  if (!Number.isSafeInteger(value)) throw new Error("invalid fixed J8 input");
  return value;
}

function main(): void {
  appendV2J8State(required("PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"), elapsedMs());
}

try {
  main();
} catch {
  process.exitCode = 1;
}
