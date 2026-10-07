import {
  PLE_QUESTION_JSON_FORMAT,
  PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND,
  type PleQuestionJsonDocument,
} from "./question_json_source";
import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { Tag } from "../../../generated/api/Tag";

/** Question-record metadata is deliberately separate from the Native JSON document. */
export type PleQuestionJsonRecordMetadata = {
  readonly questionTitle: string;
  readonly questionDescription: string;
  readonly tags: ReadonlyArray<Tag>;
  readonly questionLicense: QuestionLicense | null;
  readonly questionCitation: string | null;
  readonly language: string | null;
};

export function createDefaultPleQuestionJsonRecordMetadata(): PleQuestionJsonRecordMetadata {
  return {
    questionTitle: "",
    questionDescription: "",
    tags: [],
    questionLicense: null,
    questionCitation: null,
    language: null,
  };
}

/** Provides a complete, immediately valid starting point for a new author draft. */
export function createDefaultPleQuestionJsonSource(): PleQuestionJsonDocument {
  return {
    format: PLE_QUESTION_JSON_FORMAT,
    prompt: "Write your question prompt here.",
    response: {
      kind: PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND,
      choices: [
        { id: "choice_a", text: "First choice", feedback: null },
        { id: "choice_b", text: "Second choice", feedback: null },
      ],
      correctChoice: "choice_a",
      randomizeChoices: false,
    },
    questionHint: null,
    feedback: { correct: null, incorrect: null },
    externalResources: [],
    authorScript: null,
  };
}

/** Creates the atomic Draft record envelope accepted by the authoring create endpoint. */
export function createDefaultPleQuestionJsonDraft(): {
  readonly metadata: PleQuestionJsonRecordMetadata;
  readonly source: PleQuestionJsonDocument;
} {
  return {
    metadata: createDefaultPleQuestionJsonRecordMetadata(),
    source: createDefaultPleQuestionJsonSource(),
  };
}
