/// <reference types="node" />
import { defineConfig } from "@playwright/test";
import { resolve } from "node:path";

import { liveDemoInputsFromEnvironment } from "./tests/playwright/browser_suite_live_config";
import { liveDemoChromiumArgs } from "./tests/playwright/helper_gateway_trust.mjs";

export interface ProductionBrowserUse {
  readonly baseURL: string;
  readonly launchOptions: { readonly args: string[] };
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
  return { baseURL: input.baseUrl, launchOptions: { args: liveDemoChromiumArgs(input.baseUrl) } };
}

/** Shared only by real-stack scenarios after the configuration accepted owner input. */
export const configuredLiveDemoInputs = requireOwnerInput(process.env);
const browserUse = productionBrowserUse(process.env);

function exactTemporarySpecMatch(specPath: string): RegExp {
  const absolutePath = resolve(specPath);
  // ASVS 1.2.9: escape the trusted path before using it as a regular expression.
  const escapedPath = absolutePath.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  return new RegExp(`^${escapedPath}$`, "u");
}

export default defineConfig({
  testDir:
    configuredLiveDemoInputs.temporarySpecPath === undefined ? "tests/playwright/e2e" : "tests",
  ...(configuredLiveDemoInputs.temporarySpecPath === undefined
    ? {}
    : {
        testMatch: exactTemporarySpecMatch(configuredLiveDemoInputs.temporarySpecPath),
      }),
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  outputDir: "test-results",
  use: {
    baseURL: browserUse.baseURL,
    headless: true,
    launchOptions: browserUse.launchOptions,
  },
});
