// MOD-UI-GRADEBOOK behavior checks: one compact initial projection, opt-in history.

import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { loadGradebookPage, loadGradebookRunHistory } from "../src/pages/gradebook_page_model.ts";
import { formatPercentScore, formatPointScore, formatScoreValue } from "../src/score_format.ts";

function spyClient() {
  const calls = [];
  return {
    calls,
    client: {
      listGradebook: async (...arguments_) => {
        calls.push(["listGradebook", ...arguments_]);
        return { items: publishedProblemFixture.gradebook, nextCursor: null };
      },
      listRuns: async (...arguments_) => {
        calls.push(["listRuns", ...arguments_]);
        return { items: publishedProblemFixture.runs, nextCursor: "older-runs" };
      },
      getEnrollment: () => {
        throw new Error("gradebook must not fetch enrollments per row");
      },
      getRun: () => {
        throw new Error("gradebook must not fetch runs per row");
      },
      listAttempts: () => {
        throw new Error("gradebook must not fetch attempts per row");
      },
      getSummary: () => {
        throw new Error("gradebook must not refetch summaries per row");
      },
    },
  };
}

test("initial gradebook load is exactly one compact request even for a 30-row projection", async () => {
  const { client, calls } = spyClient();
  const thirtyRows = Array.from({ length: 30 }, () => publishedProblemFixture.gradebook[0]);
  client.listGradebook = async (...arguments_) => {
    calls.push(["listGradebook", ...arguments_]);
    return { items: thirtyRows, nextCursor: null };
  };

  const page = await loadGradebookPage(client, publishedProblemFixture.course.id);

  assert.equal(page.items.length, 30);
  assert.deepEqual(calls, [["listGradebook", publishedProblemFixture.course.id]]);
});

test("run history remains lazy and passes its cursor without refetching the gradebook", async () => {
  const { client, calls } = spyClient();
  const enrollmentId = publishedProblemFixture.enrollment.id;

  await loadGradebookPage(client, publishedProblemFixture.course.id);
  const history = await loadGradebookRunHistory(client, enrollmentId, "older-runs");

  assert.equal(history.nextCursor, "older-runs");
  assert.deepEqual(calls, [
    ["listGradebook", publishedProblemFixture.course.id],
    ["listRuns", enrollmentId, "older-runs"],
  ]);
});

test("score display trims artifacts and rounds exact midpoints away from zero", () => {
  assert.equal(formatScoreValue(8.0000000000006), "8");
  assert.equal(formatScoreValue(8.5), "8.5");
  assert.equal(formatScoreValue(8.333), "8.33");
  assert.equal(formatPointScore(8.0000000000006, 10), "8 / 10");
  assert.equal(formatPercentScore(0.080000000000006), "8%");
  assert.equal(formatPercentScore(0.083333), "8.33%");
  assert.equal(formatPercentScore(0.00125), "0.13%");
  assert.equal(formatPercentScore(-0.00125), "-0.13%");
  assert.equal(formatPercentScore(0.83335), "83.34%");
  assert.equal(formatPercentScore(-0.83335), "-83.34%");
  assert.equal(formatPercentScore(null), "-");
  assert.equal(formatScoreValue(-0.00001), "0");
  assert.throws(() => formatScoreValue(Number.NaN), /score must be finite/);
});

test("gradebook page renders only compact projections and keeps sensitive learning material out", () => {
  const source = fs.readFileSync("src/pages/gradebook_page.tsx", "utf8");
  const model = fs.readFileSync("src/pages/gradebook_page_model.ts", "utf8");

  assert.match(source, /loadGradebookPage\(runtime\.client, courseId\)/);
  assert.match(source, /loadGradebookRunHistory\(runtime\.client, enrollmentId, cursor\)/);
  assert.match(model, /return client\.listGradebook\(courseId\)/);
  assert.match(model, /return client\.listRuns\(enrollmentId, cursor\)/);
  assert.doesNotMatch(source, /\.getEnrollment\(/);
  assert.doesNotMatch(source, /\.getSummary\(/);
  assert.doesNotMatch(source, /\.listAttempts\(/);
  assert.doesNotMatch(source, /\.getAttempt\(/);
  assert.doesNotMatch(source, /\.getIssuedQuestion\(/);
  assert.doesNotMatch(source, /answer(?:Key)?|correctResponse|solution|grading|feedback/i);
});
