// WP-C9 contract tests for routes, typed mock client, and answer secrecy.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";
import { ROUTE_CONTRACT } from "../src/route_contract.ts";

const EXPECTED_ROUTE_PATHS = [
  "/",
  "/courses/:courseId",
  "/courses/:courseId/assignments/:assignmentId",
  "/runs/:runId",
  "/runs/:runId/summary",
  "/library",
  "/library/:problemId/versions/:versionId",
  "/workspace",
  "/workspace/:workspaceId",
  "/instructor/courses/:courseId/assignments/:assignmentId/edit",
  "/instructor/courses/:courseId/gradebook",
];

test("the product route data matches the frozen eleven-route contract", () => {
  assert.deepEqual(
    ROUTE_CONTRACT.map((route) => route.path),
    EXPECTED_ROUTE_PATHS,
  );
  assert.equal(new Set(ROUTE_CONTRACT.map((route) => route.id)).size, ROUTE_CONTRACT.length);
});

test("the typed mock client loads a complete run screen with no backend", async () => {
  const client = createMockApiClient();
  const activeRun = await client.startRun(publishedProblemFixture.assignment.id);
  const screen = await client.getRunScreen(activeRun.id);

  assert.equal(screen.run.completedAt, null);
  assert.equal(screen.attempt.run, activeRun.id);
  assert.equal(screen.attempt.seed, 1004);
  assert.equal(screen.question.version, screen.attempt.questionVersion);
  assert.equal(screen.question.response.kind, "multipleChoice");
});

test("mock fallback validation reports shape only and never correctness", async () => {
  const client = createMockApiClient();
  const definition = publishedProblemFixture.publishedProblem.response;
  assert.equal(definition.kind, "multipleChoice");
  if (definition.kind !== "multipleChoice") {
    return;
  }

  const empty = await client.validateResponseFormatOnServer(definition, {
    kind: "multipleChoice",
    selected: [],
  });
  assert.deepEqual(empty.violations, [
    { kind: "selectionCount", expected: { kind: "exactlyOne" }, actual: 0 },
  ]);

  const structurallyValid = await client.validateResponseFormatOnServer(definition, {
    kind: "multipleChoice",
    selected: ["carbonyl"],
  });
  assert.deepEqual(structurallyValid, { violations: [] });
  assert.equal("correct" in structurallyValid, false);
});

test("mock timer fallback follows inclusive grace and pause boundaries", async () => {
  const client = createMockApiClient();
  const policy = { kind: "perQuestion", seconds: 9, graceSeconds: 2 };
  const cases = [
    {
      name: "pause extension keeps a submission on time",
      submittedAt: 11_500,
      evaluatedAt: 11_500,
      pauseExtensionMillis: 2_000,
      expected: "submittedOnTime",
    },
    {
      name: "inclusive grace boundary remains acceptable",
      submittedAt: 12_000,
      evaluatedAt: 12_000,
      pauseExtensionMillis: 0,
      expected: "submittedWithinGrace",
    },
    {
      name: "one millisecond after grace times out",
      submittedAt: 12_001,
      evaluatedAt: 12_001,
      pauseExtensionMillis: 0,
      expected: "timedOut",
    },
  ];

  for (const timerCase of cases) {
    const verdict = await client.timerVerdictOnServer({
      policy,
      timer: {
        issuedAt: 1_000,
        deadline: 10_000,
        submittedAt: timerCase.submittedAt,
      },
      evaluatedAt: timerCase.evaluatedAt,
      pauseExtensionMillis: timerCase.pauseExtensionMillis,
    });
    assert.equal(verdict, timerCase.expected, timerCase.name);
  }
});

test("mock capability fallback names every missing requirement", async () => {
  const client = createMockApiClient();
  const violations = await client.validateAssignmentConfigOnServer({
    questions: [
      {
        question: publishedProblemFixture.publishedProblem,
        backendCapabilities: [],
      },
    ],
    requiredCapabilities: ["printExport"],
  });

  assert.deepEqual(
    violations.map((violation) => violation.capability),
    ["algorithmicGeneration", "serverGrading", "printExport"],
  );
  assert.ok(
    violations.every(
      (violation) => violation.question === publishedProblemFixture.publishedProblem.version,
    ),
  );
});

test("the generated browser surface contains no answer-bearing type", () => {
  const apiDirectory = path.resolve("generated/api");
  const files = fs.readdirSync(apiDirectory).filter((filename) => filename.endsWith(".ts"));
  const forbiddenFilename = /answer[_-]?key|correct[_-]?response|solution[_-]?key/i;
  const forbiddenType = /\b(?:AnswerKey|CorrectResponse|SolutionKey)\b/;
  const forbiddenBoundary = /crates\/grading|\.\.\/grading|grading::/;

  assert.ok(files.length > 0, "generated API surface is empty");
  for (const filename of files) {
    assert.doesNotMatch(filename, forbiddenFilename);
    const source = fs.readFileSync(path.join(apiDirectory, filename), "utf8");
    assert.doesNotMatch(source, forbiddenType, filename);
    assert.doesNotMatch(source, forbiddenBoundary, filename);
  }
});
