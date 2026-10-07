// Compiles the ordinary Pool editor for an isolated browser contract.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export async function bundleQuestionPoolMembersEditorSortHarness(): Promise<{
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}> {
  try {
    const result = await build({
      bundle: true,
      external: ["/assets/fonts/*"],
      entryPoints: [
        new URL("./question_pool_members_editor_sort_harness.tsx", import.meta.url).pathname,
      ],
      format: "iife",
      globalName: "PleQuestionPoolMembersEditorSortHarness",
      outfile: "question_pool_members_editor_sort_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined) {
      throw new Error("Question Pool member editor bundle is missing JavaScript or CSS.");
    }
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
