// Answer-free Student landing data and server-owned decision wording.

import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

const bundle = await build({
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
const javascript = bundle.outputFiles.find((output) => output.path.endsWith(".js"));
assert.ok(javascript, "Student Assessment SSR bundle must contain JavaScript");
const {
  StudentAssessmentDecisionDetails,
  toStudentAssessmentPresentationData,
  formatAssessmentActivity,
  formatAssessmentDeliveryTime,
} = await import(
  `data:text/javascript;base64,${Buffer.from(javascript.contents).toString("base64")}`
);

test("Student landing counts available Questions and Pool selections without source identities", () => {
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
      { kind: "fixedQuestion", availability: "available", questionId: "source-question" },
      { kind: "fixedQuestion", availability: "retired" },
      { kind: "fixedQuestion", availability: "available" },
      {
        kind: "questionPool",
        availability: "available",
        selectionCount: 3,
        questionPoolId: "source-pool",
      },
      { kind: "questionPool", availability: "retired", selectionCount: 2 },
    ],
  });

  assert.equal(presentation.questionsPerAssessmentAttempt, 5);
  assert.equal(presentation.delivery.studentLateWorkStatus, "on_time");
  assert.equal(presentation.displayTimeZone, "America/New_York");
  assert.equal("entries" in presentation, false);
  assert.equal("id" in presentation, false);
  assert.doesNotMatch(JSON.stringify(presentation), /A7K3M2QAS|source-question|source-pool/u);
});

test("Student decision deadlines use the supplied zone and explain nullable limits", () => {
  const dueAt = Date.parse("2026-01-15T18:30:00Z");
  const decision = {
    availableAt: null,
    dueAt,
    closesAt: null,
    timeLimitSeconds: null,
    attemptLimit: null,
    lateWorkRule: "accept",
    evaluatedAt: dueAt,
    startDecision: "may_start",
    publicReason: null,
  };
  const renderedDueTimes = [];
  for (const displayTimeZone of ["America/New_York", "America/Los_Angeles"]) {
    const formatter = new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
      timeZone: displayTimeZone,
    });
    const expectedDue = formatter.format(dueAt);
    renderedDueTimes.push(expectedDue);
    const html = renderToString(() =>
      createComponent(StudentAssessmentDecisionDetails, {
        decision: { ...decision, displayTimeZone },
      }),
    );
    assert.ok(html.includes(expectedDue), `deadline must use ${displayTimeZone}`);
    assert.match(html, /Can start/u);
    assert.match(html, /No opening time/u);
    assert.match(html, /No closing time/u);
    assert.match(html, /Unlimited Attempts/u);
    assert.match(html, /Time limit unavailable/u);
    assert.doesNotMatch(html, /America\/|time zone/iu);
    assert.equal(
      formatAssessmentActivity(dueAt, (timestamp) => formatter.format(timestamp)),
      expectedDue,
    );
    assert.equal(
      formatAssessmentDeliveryTime(dueAt, (timestamp) => formatter.format(timestamp)),
      expectedDue,
    );
  }
  assert.notEqual(renderedDueTimes[0], renderedDueTimes[1]);

  const publicReason = "This Coursework is closed for new work.";
  const closedHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, {
      decision: {
        ...decision,
        dueAt: null,
        displayTimeZone: "America/New_York",
        startDecision: "closed",
        publicReason,
      },
    }),
  );
  assert.match(closedHtml, /Cannot start/u);
  assert.match(closedHtml, /No due time/u);
  assert.ok(closedHtml.includes(publicReason));
});
