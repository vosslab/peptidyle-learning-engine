// Browser names for the cargo-tools stored Question fixture set.
//
// `published_question/fixture_set.json` is offline type-loading evidence for
// `cargo tools fixtures --check`. It is not Live Demo or installation data.
// Install inserts mint public Question IDs; the checksum-valid example ID in
// that file is not a frozen install identity.

import fixtureSet from "./published_question/fixture_set.json" with { type: "json" };

type BrowserQuestionAttempt = Omit<(typeof fixtureSet.attempts)[number], "reproductionDetails"> & {
  readonly questionPoolSelectionPosition: null;
};

type BrowserIssuedQuestion = Omit<
  (typeof fixtureSet.issuedQuestions)[number],
  "reproductionDetails" | "pointValue" | "scoringRule" | "questionPoolSelection" | "poolMember"
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
    poolMember: _poolMember,
    ...browserSafeIssuedQuestion
  } = issuedQuestion;
  // Durable issued positions are zero-based; the Student display ordinal is one-based.
  return { ...browserSafeIssuedQuestion, issuedPosition: issuedQuestion.issuedPosition + 1 };
}

export const publishedQuestionFixture = {
  sourceObjectId: fixtureSet.sourceObjectId,
  sourceObjectChecksum: fixtureSet.sourceObjectChecksum,
  publishedQuestion: fixtureSet.questionSummary,
  course: fixtureSet.course,
  assessment: fixtureSet.assessment,
  studentRecord: fixtureSet.studentRecord,
  assessment_attempts: fixtureSet.assessment_attempts,
  issuedQuestions: fixtureSet.issuedQuestions.map(browserIssuedQuestion),
  attempts: fixtureSet.attempts.map(browserQuestionAttempt),
};
