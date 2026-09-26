// record_sort_control_harness_loader.ts - compiles RecordSortControl browser evidence.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface RecordSortControlHarnessBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleRecordSortControlHarness(): Promise<RecordSortControlHarnessBundle> {
  try {
    const result = await build({
      bundle: true,
      external: ["/assets/fonts/*"],
      entryPoints: [new URL("./record_sort_control_harness.tsx", import.meta.url).pathname],
      format: "iife",
      globalName: "PleRecordSortControlHarness",
      outfile: "record_sort_control_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined) {
      throw new Error("RecordSortControl harness bundle is missing JavaScript or component CSS.");
    }
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
