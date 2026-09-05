// ribbon_component_ssr.ts - compile the real Solid component for Node SSR evidence.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import type { Component } from "solid-js";

import type { AppRibbonProps } from "../../src/ribbon/app_ribbon";

interface ComponentBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

async function bundleAppRibbon(
  platform: "browser" | "node",
  ssr: boolean,
): Promise<ComponentBundle> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("../../src/ribbon/app_ribbon.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "app_ribbon.js",
    platform,
    plugins: [solidPlugin({ solid: { generate: ssr ? "ssr" : "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
  if (javascript === undefined || stylesheet === undefined) {
    throw new Error("Solid component bundle is missing JavaScript or component CSS output.");
  }
  return { javascript: javascript.contents, stylesheet: stylesheet.text };
}

/** Loads the source component through Solid's SSR transform, not a test double. */
export async function loadAppRibbonForSsr(): Promise<Component<AppRibbonProps>> {
  const bundle = await bundleAppRibbon("node", true);
  const encoded = Buffer.from(bundle.javascript).toString("base64");
  const module: unknown = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module !== "object" || module === null || !("AppRibbon" in module)) {
    throw new Error("Solid SSR component bundle does not export AppRibbon.");
  }
  const component = module.AppRibbon;
  if (typeof component !== "function") throw new Error("AppRibbon SSR export is not a component.");
  return component as Component<AppRibbonProps>;
}

/** The browser component bundle's CSS, selected by output extension rather than output order. */
export async function bundledAppRibbonCss(): Promise<string> {
  return (await bundleAppRibbon("browser", false)).stylesheet;
}
