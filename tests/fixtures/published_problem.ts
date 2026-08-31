// Browser-facing projection of the approved cross-layer Question fixture.
//
// The stored JSON remains the single source for Question, Course, Assignment,
// and attempt data. This module only supplies browser test names for the same
// current contracts; it never carries a second serialized copy.

import fixtureSet from "./published_problem/fixture_set.json" with { type: "json" };

type BrowserQuestionAttempt = Omit<
  (typeof fixtureSet.attempts)[number],
  "parameterHash" | "reproductionDetails"
> & {
  readonly questionPoolSelection: null;
};

function browserQuestionAttempt(
  attempt: (typeof fixtureSet.attempts)[number],
): BrowserQuestionAttempt {
  const {
    parameterHash: _parameterHash,
    reproductionDetails: _reproductionDetails,
    ...browserSafeAttempt
  } = attempt;
  return { ...browserSafeAttempt, questionPoolSelection: null };
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
