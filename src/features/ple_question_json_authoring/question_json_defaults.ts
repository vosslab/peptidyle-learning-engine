import {
  PLE_QUESTION_JSON_FORMAT,
  PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND,
  PLE_QUESTION_JSON_SCHEMA_VERSION,
  type PleQuestionJsonDocument,
} from "./question_json_source";

/** Provides a complete, immediately valid starting point for a new author draft. */
export function createDefaultPleQuestionJsonSource(): PleQuestionJsonDocument {
  return {
    format: PLE_QUESTION_JSON_FORMAT,
    version: PLE_QUESTION_JSON_SCHEMA_VERSION,
    title: "Untitled question",
    questionDescription: "Instructor-facing summary of this Question.",
    prompt: "Write your question prompt here.",
    response: {
      kind: PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND,
      choices: [
        { id: "choice_a", text: "First choice", feedback: null },
        { id: "choice_b", text: "Second choice", feedback: null },
      ],
      correctChoice: "choice_a",
    },
    questionHint: null,
    feedback: { correct: null, incorrect: null },
    points: 1,
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
    tags: [],
    questionLicense: null,
    questionCitation: null,
    language: "en-US",
  };
}
