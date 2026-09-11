import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";

import {
  assignmentAttemptContext,
  assignmentAttemptHistoryData,
} from "./support/route_scope_provider_fixtures.ts";
import { ROUTE_CONTRACT } from "../src/route_contract.ts";
import { deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";

const LABELS = { accountLabel: "Student account" };
const ATTEMPT_PATH = "/assignment-attempts/R-1";
const attemptRoute = ROUTE_CONTRACT.find((route) => route.id === "assignmentAttempt");

async function loadRibbonParamsFor() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("../src/app.tsx", import.meta.url).pathname],
    external: [
      "@solidjs/router",
      "solid-js",
      "./application_shell",
      "./auth/session_context",
      "./features/course_appearance/course_theme_context",
    ],
    format: "cjs",
    outfile: "app_ribbon_params.js",
    platform: "node",
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("App Ribbon parameter bundle is missing JavaScript.");
  const module = { exports: {} };
  const require = () => ({});
  new Function("require", "module", "exports", javascript.text)(require, module, module.exports);
  if (typeof module.exports.ribbonParamsFor !== "function") {
    throw new Error("App Ribbon parameter bundle has no ribbonParamsFor export.");
  }
  return module.exports.ribbonParamsFor;
}

function backToAssignmentControl(params) {
  assert.ok(attemptRoute, "the Student Attempt route is declared");
  const model = deriveRibbonModel(
    { route: attemptRoute, params },
    { productRole: "student" },
    LABELS,
  );
  const control = model.taskAreas
    .flatMap((area) => area.controls)
    .find((candidate) => candidate.id === "backToAssignments");
  assert.ok(control, "the Attempt task row has Back to Assignments");
  return control;
}

test("resolved Student Attempt scope projects its public Assignment return path", async () => {
  const ribbonParamsFor = await loadRibbonParamsFor();
  const context = assignmentAttemptContext("C-1");
  const params = ribbonParamsFor(attemptRoute, ATTEMPT_PATH, {
    kind: "assignmentAttempt",
    context,
  });
  assert.deepEqual(params, {
    assignmentAttemptRef: "R-1",
    courseRef: "C-1",
    assignmentRef: "A-1",
  });
  assert.equal(backToAssignmentControl(params).href, "/courses/C-1/assignments/A-1");
});

test("unresolved Student Attempt scope withholds its Assignment return path", async () => {
  const ribbonParamsFor = await loadRibbonParamsFor();
  const params = ribbonParamsFor(attemptRoute, ATTEMPT_PATH, undefined);
  assert.deepEqual(params, { assignmentAttemptRef: "R-1" });
  const control = backToAssignmentControl(params);
  assert.equal(control.availability, "Unavailable");
  assert.equal(control.href, undefined);
});

test("Student Attempt history uses its authorized public return parameters", async () => {
  const ribbonParamsFor = await loadRibbonParamsFor();
  const summaryRoute = ROUTE_CONTRACT.find((route) => route.id === "assignmentAttemptSummary");
  assert.ok(summaryRoute, "the Student Attempt summary route is declared");
  const params = ribbonParamsFor(summaryRoute, "/assignment-attempts/R-1/summary", {
    kind: "assignmentAttemptHistory",
    history: assignmentAttemptHistoryData("C-1"),
  });
  assert.deepEqual(params, {
    assignmentAttemptRef: "R-1",
    courseRef: "C-1",
    assignmentRef: "A-1",
  });
});
