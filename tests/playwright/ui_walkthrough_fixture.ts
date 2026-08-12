// ui_walkthrough_fixture.ts - private input fixture for explicit live walkthrough configs.

import { test as base, expect } from "@playwright/test";

import {
  readUiWalkthroughInputs,
  type InstructorSetupInputs,
  type UiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";

interface UiWalkthroughFixtures {
  readonly uiWalkthroughInputs: UiWalkthroughInputs | undefined;
  readonly instructorSetupInputs: InstructorSetupInputs | undefined;
}

function inputPathFromMetadata(metadata: Record<string, unknown>): string | undefined {
  const inputPath = metadata["uiWalkthroughInputPath"];
  if (inputPath === undefined) return undefined;
  if (typeof inputPath !== "string" || inputPath === "") {
    throw new Error("UI walkthrough config metadata has an invalid input path");
  }
  return inputPath;
}

function learnerInputsFromMetadata(
  metadata: Record<string, unknown>,
): UiWalkthroughInputs | undefined {
  const inputPath = inputPathFromMetadata(metadata);
  if (inputPath === undefined) return undefined;
  const inputs = readUiWalkthroughInputs(inputPath);
  return inputs.stage === "learner_journey" ? inputs : undefined;
}

function setupInputsFromMetadata(
  metadata: Record<string, unknown>,
): InstructorSetupInputs | undefined {
  const inputPath = inputPathFromMetadata(metadata);
  if (inputPath === undefined) return undefined;
  const inputs = readUiWalkthroughInputs(inputPath);
  return inputs.stage === "instructor_setup" ? inputs : undefined;
}

export const test = base.extend<UiWalkthroughFixtures>({
  uiWalkthroughInputs: async ({}, use, testInfo) => {
    await use(learnerInputsFromMetadata(testInfo.project.metadata));
  },
  instructorSetupInputs: async ({}, use, testInfo) => {
    await use(setupInputsFromMetadata(testInfo.project.metadata));
  },
});

export { expect };
