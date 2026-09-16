// Browser-facing Question Details for the approved cross-layer Question fixture.
//
// The stored JSON remains the single source for Question, Course, Assessment,
// and attempt data. This module only supplies browser test names for the same
// current contracts; it never carries a second serialized copy.

import fixtureSet from "./published_question/fixture_set.json" with { type: "json" };

type BrowserQuestionAttempt = Omit<(typeof fixtureSet.attempts)[number], "reproductionDetails"> & {
  readonly questionPoolSelectionPosition: null;
};

type BrowserIssuedQuestion = Omit<
  (typeof fixtureSet.issuedQuestions)[number],
  | "reproductionDetails"
  | "pointValue"
  | "scoringRule"
  | "questionPoolSelection"
  | "poolRevisionMember"
>;

function browserQuestionAttempt(
  attempt: (typeof fixtureSet.attempts)[number],
): BrowserQuestionAttempt {
  const { reproductionDetails: _reproductionDetails, ...browserSafeAttempt } = attempt;
  return { ...browserSafeAttempt, questionPoolSelectionPosition: null };
}

function browserIssuedQuestion(
  issuedQuestion: (typeof fixtureSet.issuedQuestions)[number],
): BrowserIssuedQuestion {
  const {
    reproductionDetails: _reproductionDetails,
    pointValue: _pointValue,
    scoringRule: _scoringRule,
    questionPoolSelection: _questionPoolSelection,
    poolRevisionMember: _poolRevisionMember,
    ...browserSafeIssuedQuestion
  } = issuedQuestion;
  // Durable issued positions are zero-based; the Student display ordinal is one-based.
  return { ...browserSafeIssuedQuestion, issuedPosition: issuedQuestion.issuedPosition + 1 };
}

export const publishedQuestionFixture = {
  sourceObjectReference: fixtureSet.sourceObjectReference,
  sourceObjectChecksum: fixtureSet.sourceObjectChecksum,
  publishedQuestion: fixtureSet.questionSummary,
  course: fixtureSet.course,
  assessment: fixtureSet.assessment,
  studentRecord: fixtureSet.studentRecord,
  assessment_attempts: fixtureSet.assessment_attempts,
  issuedQuestions: fixtureSet.issuedQuestions.map(browserIssuedQuestion),
  attempts: fixtureSet.attempts.map(browserQuestionAttempt),
};
