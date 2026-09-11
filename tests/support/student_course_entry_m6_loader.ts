// Compile the Student current-Course entry harness for browser evidence.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

export interface StudentCourseEntryM6Bundle {
  readonly javascript: Uint8Array;
}

export async function bundleStudentCourseEntryM6Harness(): Promise<StudentCourseEntryM6Bundle> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./student_course_entry_m6_harness.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "student_course_entry_m6_harness.js",
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Student Course entry harness is missing JavaScript.");
  return { javascript: javascript.contents };
}
