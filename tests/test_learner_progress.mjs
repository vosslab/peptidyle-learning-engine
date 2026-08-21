// Learner score disclosure is a server fact, never a browser timing calculation.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeLearnerAssignmentProgress } from "../src/api/decoders.ts";
import { learnerProgressSummary, learnerScoreValue } from "../src/learner_progress.ts";

const available = {
  scoreState: "available",
  scoringStatus: "current",
  currentScore: 0.75,
  bestScore: 0.9,
  latestScore: 0.8,
  completedRunCount: 2,
  totalQuestionAttempts: 5,
  lastActivityAt: 1786000000000,
};

const insufficientEvidence = {
  ...available,
  classStatistics: { state: "insufficientEvidence" },
};

const classStatisticsAvailable = {
  ...available,
  classStatistics: {
    state: "available",
    completedLearnerCohortSize: 5,
    assignmentAverageScore: 0.625,
  },
};

test("learner progress is exact, key-free, and never accepts withheld totals", () => {
  assert.deepEqual(decodeLearnerAssignmentProgress(available), available);
  for (const forbidden of ["tenant", "enrollment", "policy", "evaluatedAt"]) {
    assert.throws(
      () => decodeLearnerAssignmentProgress({ ...available, [forbidden]: "private" }),
      DecodeError,
    );
  }
  assert.throws(
    () => decodeLearnerAssignmentProgress({ ...available, scoreState: "withheld" }),
    DecodeError,
  );
});

test("learner class statistics are an exact, optional safe union", () => {
  assert.deepEqual(decodeLearnerAssignmentProgress(insufficientEvidence), insufficientEvidence);
  assert.deepEqual(
    decodeLearnerAssignmentProgress(classStatisticsAvailable),
    classStatisticsAvailable,
  );

  for (const malformed of [
    { state: "insufficientEvidence", assignmentAverageScore: 0.75 },
    { state: "insufficientEvidence", completedLearnerCohortSize: 8 },
    { state: "available", completedLearnerCohortSize: 5 },
    { state: "available", completedLearnerCohortSize: 0, assignmentAverageScore: 0.75 },
    { state: "available", completedLearnerCohortSize: 1, assignmentAverageScore: 0.75 },
    { state: "available", completedLearnerCohortSize: 4, assignmentAverageScore: 0.75 },
    { state: "available", completedLearnerCohortSize: 5, assignmentAverageScore: 1.01 },
    { state: "unknown" },
  ]) {
    assert.throws(
      () => decodeLearnerAssignmentProgress({ ...available, classStatistics: malformed }),
      DecodeError,
    );
  }
});

test("learner score copy distinguishes no activity, withheld, and available nulls", () => {
  const noActivity = {
    scoreState: "noActivity",
    scoringStatus: "current",
    currentScore: null,
    bestScore: null,
    latestScore: null,
    completedRunCount: 0,
    totalQuestionAttempts: 0,
    // Starting a run records activity time, but no score activity exists until submission.
    lastActivityAt: 1786000000000,
  };
  const withheld = {
    ...noActivity,
    scoreState: "withheld",
    totalQuestionAttempts: 1,
    lastActivityAt: 1786000000000,
  };
  assert.match(learnerProgressSummary(decodeLearnerAssignmentProgress(noActivity)), /No score yet/);
  assert.match(
    learnerProgressSummary(decodeLearnerAssignmentProgress(withheld)),
    /Score is currently unavailable/,
  );
  assert.equal(learnerScoreValue(null), "No score yet");
  assert.match(learnerProgressSummary(decodeLearnerAssignmentProgress(available)), /75%/);
  assert.match(
    learnerProgressSummary(
      decodeLearnerAssignmentProgress({
        ...available,
        scoringStatus: "recalculating",
        currentScore: null,
        bestScore: null,
        latestScore: null,
      }),
    ),
    /recalculating/i,
  );
});
