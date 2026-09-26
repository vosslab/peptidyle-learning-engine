// Compiles the actual Instructor Due Soon route for browser-contract evidence.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface AssessmentsDueSoonHarnessBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleAssessmentsDueSoonHarness(): Promise<AssessmentsDueSoonHarnessBundle> {
  try {
    const result = await build({
      bundle: true,
      external: ["/assets/fonts/*"],
      entryPoints: [new URL("./assessments_due_soon_harness.tsx", import.meta.url).pathname],
      format: "iife",
      globalName: "PleAssessmentsDueSoonHarness",
      outfile: "assessments_due_soon_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined) {
      throw new Error("Assessments Due Soon harness bundle is missing JavaScript or CSS.");
    }
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
