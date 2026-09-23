// provided_avatar_picker_harness_loader.ts - bundles current picker source for browser evidence.

import { build, stop } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface ProvidedAvatarPickerHarnessBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleProvidedAvatarPickerHarness(): Promise<ProvidedAvatarPickerHarnessBundle> {
  try {
    const result = await build({
      bundle: true,
      external: ["/assets/fonts/*"],
      entryPoints: [new URL("./provided_avatar_picker_harness.tsx", import.meta.url).pathname],
      format: "esm",
      outfile: "provided_avatar_picker_harness.js",
      platform: "browser",
      plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
      write: false,
    });
    const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
    const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
    if (javascript === undefined || stylesheet === undefined) {
      throw new Error(
        "Provided avatar picker harness bundle is missing JavaScript or component CSS.",
      );
    }
    return { javascript: javascript.contents, stylesheet: stylesheet.text };
  } finally {
    stop();
  }
}
