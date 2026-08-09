import {
  FLAT_QUESTION_FORMAT,
  FLAT_QUESTION_KIND,
  FLAT_QUESTION_VERSION,
  type FlatQuestionSourceV1,
} from "./flat_question_source";

/** Provides a complete, immediately valid starting point for a new author draft. */
export function createDefaultFlatQuestionSource(): FlatQuestionSourceV1 {
  return {
    format: FLAT_QUESTION_FORMAT,
    version: FLAT_QUESTION_VERSION,
    kind: FLAT_QUESTION_KIND,
    title: "Untitled question",
    prompt: "Write your question prompt here.",
    choices: [
      { id: "choice_a", text: "First choice", feedback: null },
      { id: "choice_b", text: "Second choice", feedback: null },
    ],
    correctChoice: "choice_a",
    feedback: { correct: null, incorrect: null },
    points: 1,
    attemptPolicy: { maxAttempts: null, feedback: "immediateFull" },
    timingPolicy: { kind: "untimed" },
    tags: [],
    taxonomy: [],
    license: { kind: "allRightsReserved" },
    language: "en-US",
  };
}
