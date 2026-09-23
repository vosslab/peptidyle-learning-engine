// record_list_harness_loader.ts - compiles RecordList browser evidence from current source.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface RecordListHarnessBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleRecordListHarness(): Promise<RecordListHarnessBundle> {
  try {
    const result = await build({
      bundle: true,
      external: ["/assets/fonts/*"],
      entryPoints: [new URL("./record_list_harness.tsx", import.meta.url).pathname],
      format: "iife",
      globalName: "PleRecordListHarness",
      outfile: "record_list_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined) {
      throw new Error("RecordList harness bundle is missing JavaScript or component CSS.");
    }
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
