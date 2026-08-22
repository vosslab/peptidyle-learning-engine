/// <reference types="node" />
import { defineConfig } from "@playwright/test";

import { liveDemoInputsFromEnvironment } from "./tests/playwright/browser_suite_live_config";

export interface ProductionBrowserUse {
  readonly baseURL: string;
  readonly ignoreHTTPSErrors: true;
}

function requireOwnerInput(
  environment: Readonly<Record<string, string | undefined>>,
): NonNullable<ReturnType<typeof liveDemoInputsFromEnvironment>> {
  if (environment["PLE_LIVE_DEMO_BROWSER_REQUIRED"] !== "1") {
    throw new Error("Playwright requires the real-stack browser-suite owner input");
  }
  const input = liveDemoInputsFromEnvironment(environment);
  if (input === undefined)
    throw new Error("Playwright requires the real-stack browser-suite owner input");
  return input;
}

/** Read the private input issued by the disposable production-browser owner. */
export function productionBrowserUse(
  environment: Readonly<Record<string, string | undefined>>,
): ProductionBrowserUse {
  const input = requireOwnerInput(environment);
  return { baseURL: input.baseUrl, ignoreHTTPSErrors: true };
}

/** Shared only by real-stack scenarios after the configuration accepted owner input. */
export const configuredLiveDemoInputs = requireOwnerInput(process.env);
const browserUse = productionBrowserUse(process.env);

export default defineConfig({
  testDir: "tests/playwright/e2e",
  testIgnore: ["**/fault_handshake_worker.spec.ts"],
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  outputDir: "test-results",
  use: {
    baseURL: browserUse.baseURL,
    headless: true,
    ignoreHTTPSErrors: browserUse.ignoreHTTPSErrors,
  },
});
