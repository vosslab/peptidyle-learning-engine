/// <reference types="node" />
import { defineConfig } from "@playwright/test";

import { liveModeActivationFromEnvironment } from "./tests/playwright/live_mode_activation";
import { liveInputsFromEnvironment } from "./tests/playwright/webwork_live_config";

const PORT = process.env["PW_PORT"] ?? "4173";

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
  testIgnore: ["**/_temp*", "**/dist_*/**"],
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
