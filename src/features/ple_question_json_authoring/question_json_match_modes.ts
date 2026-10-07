import type { PleQuestionJsonTextResponseMatchRule } from "./question_json_source";

export const PLE_QUESTION_JSON_TEXT_RESPONSE_MATCH_MODES: ReadonlyArray<{
  readonly value: PleQuestionJsonTextResponseMatchRule;
  readonly label: string;
}> = [
  { value: "exact", label: "Exact text" },
  { value: "caseInsensitive", label: "Ignore capitalization" },
  { value: "normalized", label: "Ignore capitalization and ordinary spacing" },
  { value: "regex", label: "Regular expression" },
];
