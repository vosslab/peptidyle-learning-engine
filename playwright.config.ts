/// <reference types="node" />
import { defineConfig } from "@playwright/test";

import { liveInputsFromEnvironment } from "./tests/playwright/webwork_live_config";

const PORT = process.env["PW_PORT"] ?? "4173";

/** The live WebWork gate drives a running private stack, never the mock preview server. */
export function mockPreviewServerEnabled(
  environment: Readonly<Record<string, string | undefined>>,
): boolean {
  return environment["PLE_WEBWORK_LIVE_REQUIRED"] !== "1";
}

const startMockPreviewServer = mockPreviewServerEnabled(process.env);
/** Evaluated while Playwright loads this file, before it can create Chromium. */
export const configuredLiveWebworkInputs = liveInputsFromEnvironment(process.env);
const baseURL = configuredLiveWebworkInputs?.baseUrl ?? `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: "tests/playwright",
  testIgnore: ["**/_temp*", "**/dist_*/**"],
  timeout: 30_000,
  fullyParallel: true,
  reporter: "list",
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
