// Compile the Student current-Course entry harness for browser evidence.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface StudentCourseEntryM6Bundle {
  readonly javascript: Uint8Array;
  readonly stylesheet: string;
}

export async function bundleStudentCourseEntryM6Harness(): Promise<StudentCourseEntryM6Bundle> {
  const result = await build({
    bundle: true,
    external: ["/assets/fonts/*"],
    entryPoints: [new URL("./student_course_entry_m6_harness.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "student_course_entry_m6_harness.js",
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
  if (javascript === undefined || stylesheet === undefined)
    throw new Error("Student Course entry harness is missing JavaScript or component CSS.");
  return { javascript: javascript.contents, stylesheet: stylesheet.text };
}
