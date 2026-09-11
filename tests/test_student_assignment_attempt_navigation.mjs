// Student Assignment Attempt Question navigation state-matrix regression.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import { studentAssignmentAttemptQuestionStateLabel } from "../src/components/student_assignment_attempt_navigation_model.ts";

async function loadStudentAssignmentAttemptNavigationForSsr() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../src/components/student_assignment_attempt_navigation.tsx", import.meta.url)
        .pathname,
    ],
    format: "esm",
    outfile: "student_assignment_attempt_navigation.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) {
    throw new Error("Student Assignment Attempt navigation SSR bundle is missing JavaScript.");
  }
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.StudentAssignmentAttemptNavigation !== "function") {
    throw new Error("Student Assignment Attempt navigation SSR bundle has no component export.");
  }
  return module.StudentAssignmentAttemptNavigation;
}

test("Student Question navigation states use truthful Student-facing non-color labels", () => {
  assert.equal(studentAssignmentAttemptQuestionStateLabel("unanswered"), "Not answered");
  assert.equal(studentAssignmentAttemptQuestionStateLabel("saved"), "Saved");
  assert.equal(studentAssignmentAttemptQuestionStateLabel("closed"), "Closed");
});

test("Student Question navigation renders ordered, answer-free states with one current Question", async () => {
  const StudentAssignmentAttemptNavigation = await loadStudentAssignmentAttemptNavigationForSsr();
  const positions = [
    {
      position: 2,
      responseState: "saved",
      questionTitle: "Private title",
      answer: "private answer",
    },
    { position: 1, responseState: "unanswered", questionId: "AAA-BBBB" },
    { position: 3, responseState: "closed", privateData: "private data" },
  ];
  const html = renderToString(() =>
    createComponent(StudentAssignmentAttemptNavigation, {
      positions,
      currentPosition: 1,
      onPositionActivate: () => undefined,
    }),
  );

  assert.ok(html.indexOf("Question 2") < html.indexOf("Question 1"));
  assert.ok(html.indexOf("Question 1") < html.indexOf("Question 3"));
  assert.match(html, /Question 2[\s\S]*Saved/u);
  assert.match(html, /Question 1[\s\S]*Not answered[\s\S]*Current/u);
  assert.match(html, /Question 3[\s\S]*Closed/u);
  assert.equal((html.match(/aria-current="step"/gu) ?? []).length, 1);
  assert.equal((html.match(/Current/gu) ?? []).length, 1);
  assert.match(html, /Question 3: Closed/u);
  assert.match(html, /disabled(?:\s|>)/u);
  assert.doesNotMatch(html, /Private title|AAA-BBBB|private answer|private data/u);
});

test("Student Question navigation renders its intentional empty state", async () => {
  const StudentAssignmentAttemptNavigation = await loadStudentAssignmentAttemptNavigationForSsr();
  const html = renderToString(() =>
    createComponent(StudentAssignmentAttemptNavigation, {
      positions: [],
      currentPosition: null,
      onPositionActivate: () => undefined,
    }),
  );

  assert.match(html, /Questions will appear here when ready\./u);
  assert.doesNotMatch(html, /<button/u);
});
