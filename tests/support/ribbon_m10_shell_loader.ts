// ribbon_m10_shell_loader.ts - compiles the real application-shell composition
// for browser evidence.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface RibbonM10ShellBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleRibbonM10ShellHarness(): Promise<RibbonM10ShellBundle> {
  try {
    const result = await build({
      bundle: true,
      entryPoints: [new URL("./ribbon_m10_shell_harness.tsx", import.meta.url).pathname],
      // A classic IIFE lets the browser oracle inject the compiled application
      // directly.  Importing a megabyte-scale nested data URL delays module
      // loading enough to obscure actual mount failures in Chromium.
      format: "iife",
      globalName: "PleRibbonM10Harness",
      outfile: "ribbon_m10_shell_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined) {
      throw new Error("Application-shell harness bundle is missing JavaScript or component CSS.");
    }
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
