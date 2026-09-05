// ribbon_m11_deferred_content_loader.ts - compiles browser-only deferred-route evidence.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface RibbonM11DeferredContentBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleM11Harness(): Promise<RibbonM11DeferredContentBundle> {
  try {
    const result = await build({
      bundle: true,
      entryPoints: [new URL("./ribbon_m11_deferred_content_harness.tsx", import.meta.url).pathname],
      format: "iife",
      globalName: "PleRibbonM11DeferredContent",
      outfile: "ribbon_m11_deferred_content_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined)
      throw new Error("Deferred-content harness bundle is missing JavaScript or CSS.");
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
