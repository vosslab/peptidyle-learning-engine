// Shared answer-free Student landing behavior checks.

import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import {
  formatAssessmentAttemptTimeLimit,
  formatAssessmentActivity,
  formatAssessmentDeliveryTime,
  toStudentAssessmentPresentationData,
} from "../src/components/student_assessment_presentation.tsx";

async function loadDecisionDetailsForSsr() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../src/components/student_assessment_presentation.tsx", import.meta.url).pathname,
    ],
    format: "esm",
    outfile: "student_assessment_presentation.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Student Assessment decision SSR bundle is missing.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.StudentAssessmentDecisionDetails !== "function") {
    throw new Error("Student Assessment decision component export is missing.");
  }
  return module.StudentAssessmentDecisionDetails;
}

test("Student detail adapts available entries and Question Pool selections without exposing source identities", () => {
  const presentation = toStudentAssessmentPresentationData({
    id: "assessment-1",
    reference: "A7K3M2Q",
    title: "Protein structure",
    instructions: "Use your notes.",
    display_time_zone: "America/New_York",
    delivery: {
      available_at: null,
      due_at: null,
      closes_at: null,
      assessment_attempt_time_limit_seconds: 900,
      attempt_limit: 2,
      late_work_rule: "accept",
      student_late_work_status: "on_time",
    },
    entries: [
      { kind: "fixedQuestion", availability: "available" },
      { kind: "fixedQuestion", availability: "retired" },
      { kind: "fixedQuestion", availability: "available" },
      { kind: "questionPool", availability: "available", selectionCount: 3 },
      { kind: "questionPool", availability: "retired", selectionCount: 2 },
    ],
  });

  assert.equal(presentation.questionsPerAssessmentAttempt, 5);
  assert.equal(presentation.delivery.studentLateWorkStatus, "on_time");
  assert.equal(presentation.displayTimeZone, "America/New_York");
  assert.equal("timeZone" in presentation, false);
  assert.equal("id" in presentation, false);
});

test("attempt-time copy stays readable across minute, hour, and second limits", () => {
  assert.equal(formatAssessmentAttemptTimeLimit(3_600), "1 hour per attempt");
  assert.equal(formatAssessmentAttemptTimeLimit(90), "90 seconds per attempt");
});

test("assessment instants use the supplied viewer zone instead of the browser zone", () => {
  const timestamp = Date.parse("2026-01-15T18:30:00Z");
  const newYork = "America/New_York";
  const losAngeles = "America/Los_Angeles";
  const options = { dateStyle: "medium", timeStyle: "short", timeZone: newYork };
  const expected = new Intl.DateTimeFormat(undefined, options).format(new Date(timestamp));

  assert.equal(formatAssessmentDeliveryTime(timestamp, newYork), expected);
  assert.equal(formatAssessmentActivity(timestamp, newYork), expected);
  assert.notEqual(
    formatAssessmentDeliveryTime(timestamp, losAngeles),
    expected,
    "fixed instant must render in the supplied viewer zone",
  );
});

test("decision presentation renders one server instant differently in two supplied zones", async () => {
  const StudentAssessmentDecisionDetails = await loadDecisionDetailsForSsr();
  const dueAt = Date.parse("2026-01-15T18:30:00Z");
  const decision = {
    availableAt: Date.parse("2026-01-15T17:30:00Z"),
    dueAt,
    closesAt: Date.parse("2026-01-15T19:30:00Z"),
    timeLimitSeconds: 900,
    attemptLimit: 2,
    lateWorkRule: "reject",
    displayTimeZone: "America/New_York",
    evaluatedAt: Date.parse("2026-01-15T17:00:00Z"),
    startDecision: "may_start",
    publicReason: null,
  };
  const newYorkDue = formatAssessmentDeliveryTime(dueAt, "America/New_York");
  const losAngelesDue = formatAssessmentDeliveryTime(dueAt, "America/Los_Angeles");
  const newYorkHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, { decision }),
  );
  const losAngelesHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, {
      decision: { ...decision, displayTimeZone: "America/Los_Angeles" },
    }),
  );

  assert.notEqual(newYorkDue, losAngelesDue);
  assert.ok(newYorkHtml.includes(newYorkDue));
  assert.ok(losAngelesHtml.includes(losAngelesDue));
  assert.match(newYorkHtml, /Can start/u);
  assert.match(newYorkHtml, /Times are shown in your time zone: America\/New_York\./u);
  assert.doesNotMatch(newYorkHtml, /Cannot start/u);

  const closedReason = "This Assessment is closed for new work.";
  const closedHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, {
      decision: { ...decision, startDecision: "closed", publicReason: closedReason },
    }),
  );
  assert.match(closedHtml, /Cannot start/u);
  assert.equal((closedHtml.match(new RegExp(closedReason, "gu")) ?? []).length, 1);
});
