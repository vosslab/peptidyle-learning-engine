// ribbon_design_fixture_loader.ts - compiles the static laboratory for SSR and browser evidence.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import type { Component } from "solid-js";

interface FixtureBundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

async function bundleFixture(platform: "browser" | "node", ssr: boolean): Promise<FixtureBundle> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./ribbon_design_fixture.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "ribbon_design_fixture.js",
    platform,
    plugins: [solidPlugin({ solid: { generate: ssr ? "ssr" : "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
  if (javascript === undefined || stylesheet === undefined) {
    throw new Error("Ribbon design fixture bundle is missing JavaScript or CSS.");
  }
  return { javascript: javascript.contents, stylesheet: stylesheet.text };
}

/** Loads the fixture through the production Solid transform rather than a markup substitute. */
export async function loadRibbonDesignFixtureForSsr(): Promise<Component> {
  const bundle = await bundleFixture("node", true);
  const encoded = Buffer.from(bundle.javascript).toString("base64");
  const module: unknown = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module !== "object" || module === null || !("RibbonDesignFixture" in module)) {
    throw new Error("Ribbon design fixture SSR bundle has no RibbonDesignFixture export.");
  }
  const component = module.RibbonDesignFixture;
  if (typeof component !== "function")
    throw new Error("Ribbon design fixture export is not a component.");
  return component as Component;
}

/** Returns the complete bundled laboratory stylesheet, including both treatment paths. */
export async function bundledRibbonDesignFixtureCss(): Promise<string> {
  return (await bundleFixture("browser", false)).stylesheet;
}
