// Browser-facing projection of the approved cross-layer Question fixture.
//
// The stored JSON remains the single source for Question, Course, Assignment,
// and attempt data. This module only supplies browser test names for the same
// current contracts; it never carries a second serialized copy.

import fixtureSet from "./published_problem/fixture_set.json" with { type: "json" };

function browserQuestionAttempt({
  questionPoolEntry,
  parameterHash: _parameterHash,
  reproductionDetails: _reproductionDetails,
  ...attempt
}) {
  return { ...attempt, questionPoolSelection: questionPoolEntry ?? null };
}

export const publishedProblemFixture = {
  publishedQuestion: fixtureSet.catalogQuestion,
  publishedProblem: fixtureSet.publishedProblem,
  draft: fixtureSet.draft,
  course: fixtureSet.course,
  assignment: fixtureSet.assignment,
  studentRecord: fixtureSet.studentRecord,
  runs: fixtureSet.runs,
  issuedQuestions: fixtureSet.issuedQuestions,
  attempts: fixtureSet.attempts.map(browserQuestionAttempt),
};
