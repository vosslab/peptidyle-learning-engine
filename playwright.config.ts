/// <reference types="node" />
import { defineConfig } from "@playwright/test";

import { liveModeActivationFromEnvironment } from "./tests/playwright/live_mode_activation";
import { liveInputsFromEnvironment } from "./tests/playwright/webwork_live_config";

const PORT = process.env["PW_PORT"] ?? "4173";

const WALKTHROUGH_TEST_IGNORE = [
  "**/ui_walkthrough_instructor_setup.spec.ts",
  "**/ui_walkthrough_smoke.spec.ts",
  "**/ui_walkthrough_keyboard_*.spec.ts",
] as const;

/**
 * Ordinary discovery owns durable mock-backed tests only. Private live-stack,
 * artifact, and walkthrough lanes have dedicated launchers with explicit
 * inputs; collecting them here would report an intentional skip as success.
 */
export function testIgnoreFromEnvironment(
  environment: Readonly<Record<string, string | undefined>>,
): string[] {
  const activation = liveModeActivationFromEnvironment(environment);
  const ignored = ["**/_temp*", "**/dist_*/**", ...WALKTHROUGH_TEST_IGNORE];
  if (!activation.webwork) {
    ignored.push("**/chapter_one_run.spec.ts");
  }
  if (environment["PLE_CAPTURE_COURSE_APPEARANCE_VISUALS"] !== "1") {
    ignored.push("**/course_appearance_visual.spec.ts");
  }
  if (environment["PLE_INSTRUCTOR_PAGE_VISUALS_DIR"] === undefined) {
    ignored.push("**/instructor_page_visuals.spec.ts");
  }
  return ignored;
}

/** The live WebWork gate drives a running private stack, never the mock preview server. */
export function mockPreviewServerEnabled(
  environment: Readonly<Record<string, string | undefined>>,
): boolean {
  const activation = liveModeActivationFromEnvironment(environment);
  return !activation.webwork;
}

const liveModeActivation = liveModeActivationFromEnvironment(process.env);
/** Evaluated while Playwright loads this file, before it can create Chromium. */
export const configuredLiveWebworkInputs = liveModeActivation.webwork
  ? liveInputsFromEnvironment(process.env)
  : undefined;
const startMockPreviewServer = mockPreviewServerEnabled(process.env);
const baseURL = configuredLiveWebworkInputs?.baseUrl ?? `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: "tests/playwright",
  testIgnore: testIgnoreFromEnvironment(process.env),
  timeout: 30_000,
  fullyParallel: true,
  reporter: "list",
  outputDir: "test-results",
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
