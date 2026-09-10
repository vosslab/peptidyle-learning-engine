// Compile the Course Appearance page harness for browser evidence.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface CourseAppearanceM7Bundle {
  readonly javascript: Uint8Array;
}

export async function bundleCourseAppearanceM7Harness(): Promise<CourseAppearanceM7Bundle> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./course_appearance_m7_harness.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "course_appearance_m7_harness.js",
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Course Appearance harness is missing JavaScript.");
  return { javascript: javascript.contents };
}
