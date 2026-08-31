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
  readonly questionPoolSelectionPosition: null;
};

type BrowserIssuedQuestion = Omit<
  (typeof fixtureSet.issuedQuestions)[number],
  "questionPoolSelection" | "questionPoolCandidate"
>;

function browserQuestionAttempt(
  attempt: (typeof fixtureSet.attempts)[number],
): BrowserQuestionAttempt {
  const {
    parameterHash: _parameterHash,
    reproductionDetails: _reproductionDetails,
    ...browserSafeAttempt
  } = attempt;
  return { ...browserSafeAttempt, questionPoolSelectionPosition: null };
}

function browserIssuedQuestion(
  issuedQuestion: (typeof fixtureSet.issuedQuestions)[number],
): BrowserIssuedQuestion {
  const {
    questionPoolSelection: _questionPoolSelection,
    questionPoolCandidate: _questionPoolCandidate,
    ...browserSafeIssuedQuestion
  } = issuedQuestion;
  return browserSafeIssuedQuestion;
}

export const publishedProblemFixture = {
  publishedQuestion: fixtureSet.catalogQuestion,
  publishedProblem: fixtureSet.publishedProblem,
  draft: fixtureSet.draft,
  course: fixtureSet.course,
  assignment: fixtureSet.assignment,
  studentRecord: fixtureSet.studentRecord,
  runs: fixtureSet.runs,
  issuedQuestions: fixtureSet.issuedQuestions.map(browserIssuedQuestion),
  attempts: fixtureSet.attempts.map(browserQuestionAttempt),
};
