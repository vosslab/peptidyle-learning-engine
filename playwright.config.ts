/// <reference types="node" />
import { defineConfig } from "@playwright/test";

import { liveModeActivationFromEnvironment } from "./tests/playwright/live_mode_activation";
import { liveDemoInputsFromEnvironment } from "./tests/playwright/live_demo_live_config";
import { liveInputsFromEnvironment } from "./tests/playwright/webwork_live_config";

const PORT = process.env["PW_PORT"] ?? "4173";

const WALKTHROUGH_TEST_IGNORE = [
  "**/ui_walkthrough_instructor_setup.spec.ts",
  "**/ui_walkthrough_smoke.spec.ts",
  "**/ui_walkthrough_keyboard_*.spec.ts",
] as const;

/**
 * Ordinary discovery owns browser-test coverage. Private live-stack,
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
  if (!activation.liveDemo) ignored.push("**/e2e/live_demo.spec.ts");
  if (environment["PLE_WP_R2_WEBWORK_QUESTION_ID"] === undefined) {
    ignored.push("**/e2e/wp_r2_host_seed_renderer.spec.ts");
  }
  if (environment["PLE_CAPTURE_COURSE_APPEARANCE_VISUALS"] !== "1") {
    ignored.push("**/course_appearance_visual.spec.ts");
  }
  if (environment["PLE_INSTRUCTOR_PAGE_VISUALS_DIR"] === undefined) {
    ignored.push("**/instructor_page_visuals.spec.ts");
    ignored.push("**/t2_visual_corpus.spec.ts");
  }
  return ignored;
}

/** The live WebWork gate drives a running private stack, never the browser-test server. */
export function browserTestServerEnabled(
  environment: Readonly<Record<string, string | undefined>>,
): boolean {
  const activation = liveModeActivationFromEnvironment(environment);
  return !activation.webwork && !activation.liveDemo;
}

const liveModeActivation = liveModeActivationFromEnvironment(process.env);
/** Evaluated while Playwright loads this file, before it can create Chromium. */
export const configuredLiveWebworkInputs = liveModeActivation.webwork
  ? liveInputsFromEnvironment(process.env)
  : undefined;
export const configuredLiveDemoInputs = liveModeActivation.liveDemo
  ? liveDemoInputsFromEnvironment(process.env)
  : undefined;
const startBrowserTestServer = browserTestServerEnabled(process.env);
const baseURL =
  configuredLiveDemoInputs?.baseUrl ??
  configuredLiveWebworkInputs?.baseUrl ??
  `http://127.0.0.1:${PORT}`;

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
    // The self-signed Caddy certificate is accepted only by this selected
    // disposable live-demo lane; ordinary and WebWork lanes retain normal TLS.
    ignoreHTTPSErrors: configuredLiveDemoInputs !== undefined,
  },
  webServer: startBrowserTestServer
    ? {
        command: `node tests/playwright/helper_browser_test_server.mjs ${PORT}`,
        url: `http://127.0.0.1:${PORT}/`,
        reuseExistingServer: false,
        timeout: 30_000,
      }
    : undefined,
});
