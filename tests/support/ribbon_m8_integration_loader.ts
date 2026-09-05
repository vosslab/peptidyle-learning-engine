// ribbon_m8_integration_loader.ts - compiles the real AppRibbon harness for browser evidence.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface RibbonM8IntegrationBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

/** Uses the production Solid DOM transform rather than a hand-authored HTML substitute. */
export async function bundleRibbonM8IntegrationHarness(): Promise<RibbonM8IntegrationBundle> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./ribbon_m8_integration_harness.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "ribbon_m8_integration_harness.js",
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
  if (javascript === undefined || stylesheet === undefined) {
    throw new Error("Ribbon interaction harness bundle is missing JavaScript or Ribbon CSS.");
  }
  return { javascript: javascript.contents, stylesheet: stylesheet.text };
}
