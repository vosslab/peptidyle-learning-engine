// ribbon_m9_responsive_loader.ts - compiles the real responsive Ribbon harness.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface RibbonM9ResponsiveBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleRibbonM9ResponsiveHarness(): Promise<RibbonM9ResponsiveBundle> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./ribbon_m9_responsive_harness.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "ribbon_m9_responsive_harness.js",
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
  if (javascript === undefined || stylesheet === undefined) {
    throw new Error("Responsive harness bundle is missing JavaScript or Ribbon CSS.");
  }
  return { javascript: javascript.contents, stylesheet: stylesheet.text };
}
