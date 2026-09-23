// Shared answer-free Student landing behavior checks.

import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

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
  return module;
}

test("Student detail adapts available entries and Question Pool selections without exposing source identities", async () => {
  const { toStudentAssessmentPresentationData } = await loadDecisionDetailsForSsr();
  const presentation = toStudentAssessmentPresentationData({
    id: "A7K3M2QAS",
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

test("base duration defaults calculate from Questions while explicit and legacy durations remain stored", async () => {
  const { formatAssessmentAttemptTimeLimit } = await loadDecisionDetailsForSsr();
  assert.equal(formatAssessmentAttemptTimeLimit(null, 3), "5 minutes per attempt");
  assert.equal(formatAssessmentAttemptTimeLimit(3_600), "1 hour per attempt");
  assert.equal(formatAssessmentAttemptTimeLimit(90), "90 seconds per attempt");
});

test("assessment instants use the supplied viewer zone instead of the browser zone", async () => {
  const { formatAssessmentActivity, formatAssessmentDeliveryTime } =
    await loadDecisionDetailsForSsr();
  const timestamp = Date.parse("2026-01-15T18:30:00Z");
  const newYork = "America/New_York";
  const losAngeles = "America/Los_Angeles";
  const newYorkDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: newYork,
  });
  const losAngelesDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: losAngeles,
  });
  const newYorkFormatter = (timestamp) => newYorkDateTimeFormatter.format(timestamp);
  const losAngelesFormatter = (timestamp) => losAngelesDateTimeFormatter.format(timestamp);
  const expected = newYorkDateTimeFormatter.format(new Date(timestamp));

  assert.equal(formatAssessmentDeliveryTime(timestamp, newYorkFormatter), expected);
  assert.equal(formatAssessmentActivity(timestamp, newYorkFormatter), expected);
  assert.equal(formatAssessmentDeliveryTime(null, newYorkFormatter, "No due time"), "No due time");
  assert.equal(formatAssessmentActivity(null, newYorkFormatter), "No activity yet");
  assert.notEqual(
    formatAssessmentDeliveryTime(timestamp, losAngelesFormatter),
    expected,
    "fixed instant must render in the supplied viewer zone",
  );
});

test("decision presentation renders one server instant differently in two supplied zones", async () => {
  const { StudentAssessmentDecisionDetails, formatAssessmentDeliveryTime } =
    await loadDecisionDetailsForSsr();
  const dueAt = Date.parse("2026-01-15T18:30:00Z");
  const newYorkDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "America/New_York",
  });
  const losAngelesDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "America/Los_Angeles",
  });
  const newYorkFormatter = (timestamp) => newYorkDateTimeFormatter.format(timestamp);
  const losAngelesFormatter = (timestamp) => losAngelesDateTimeFormatter.format(timestamp);
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
  const newYorkDue = formatAssessmentDeliveryTime(dueAt, newYorkFormatter);
  const losAngelesDue = formatAssessmentDeliveryTime(dueAt, losAngelesFormatter);
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
  assert.match(newYorkHtml, /Times shown in America\/New_York/u);
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
