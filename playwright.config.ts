/// <reference types="node" />
import { defineConfig } from "@playwright/test";

import { liveModeActivationFromEnvironment } from "./tests/playwright/live_mode_activation";
import {
  type InstructorSetupLiveInputs,
  type UiWalkthroughLiveInputs,
  instructorSetupInputsFromEnvironment,
  uiWalkthroughInputsFromEnvironment,
} from "./tests/playwright/ui_walkthrough_live_config";
import { liveInputsFromEnvironment } from "./tests/playwright/webwork_live_config";

const PORT = process.env["PW_PORT"] ?? "4173";

/** The live WebWork gate drives a running private stack, never the mock preview server. */
export function mockPreviewServerEnabled(
  environment: Readonly<Record<string, string | undefined>>,
): boolean {
  const activation = liveModeActivationFromEnvironment(environment);
  return !activation.webwork && !activation.walkthrough;
}

const liveModeActivation = liveModeActivationFromEnvironment(process.env);
/** Evaluated while Playwright loads this file, before it can create Chromium. */
export const configuredLiveWebworkInputs = liveModeActivation.webwork
  ? liveInputsFromEnvironment(process.env)
  : undefined;
/** Evaluated while Playwright loads this file, before it can create Chromium. */
export const configuredUiWalkthroughInputs = liveModeActivation.walkthrough
  ? process.env["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY"] === "1"
    ? undefined
    : uiWalkthroughInputsFromEnvironment(process.env)
  : undefined;
/** Evaluated before Chromium; the instructor-only child deliberately has no course IDs yet. */
export const configuredInstructorSetupInputs = liveModeActivation.walkthrough
  ? instructorSetupInputsFromEnvironment(process.env)
  : undefined;
const startMockPreviewServer = mockPreviewServerEnabled(process.env);
const baseURL =
  configuredUiWalkthroughInputs?.baseUrl ??
  configuredInstructorSetupInputs?.baseUrl ??
  configuredLiveWebworkInputs?.baseUrl ??
  `http://127.0.0.1:${PORT}`;

/** Keeps walkthrough matcher artifacts in the runner-owned private state sibling. */
export function outputDirectoryForUiWalkthrough(
  inputs: UiWalkthroughLiveInputs | InstructorSetupLiveInputs | undefined,
): string {
  return inputs?.journeyArtifactsDirectory ?? "test-results";
}

const outputDir = outputDirectoryForUiWalkthrough(
  configuredUiWalkthroughInputs ?? configuredInstructorSetupInputs,
);

export default defineConfig({
  testDir: "tests/playwright",
  testIgnore: ["**/_temp*", "**/dist_*/**"],
  timeout: 30_000,
  fullyParallel: true,
  reporter: "list",
  outputDir,
  use: {
    baseURL,
    headless: true,
  },
  webServer: startMockPreviewServer
    ? {
        command: `node tools/mock_preview_server.mjs ${PORT}`,
        url: `http://127.0.0.1:${PORT}/`,
        reuseExistingServer: false,
        timeout: 30_000,
      }
    : undefined,
});
