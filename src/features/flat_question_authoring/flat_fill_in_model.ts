// flat_fill_in_model.ts - private authoring operations for one accepted text response.

import type { FlatQuestionFillInResponse, FlatQuestionTextMatchMode } from "./flat_question_source";

const MAX_TEXT_RESPONSE_CHARS = 16_384;

export type FlatFillInValidation = {
  readonly valid: boolean;
  readonly issues: Readonly<Record<string, string>>;
};

/** Gives duplicate detection one predictable authoring rule without changing saved answer text. */
export function normalizedAcceptedAnswer(value: string): string {
  return value.normalize("NFKC").trim().replace(/\s+/gu, " ").toLocaleLowerCase("en-US");
}

export function validateFillInResponse(response: FlatQuestionFillInResponse): FlatFillInValidation {
  const issues: Record<string, string> = {};
  if (
    !Number.isSafeInteger(response.maxLength) ||
    response.maxLength < 1 ||
    response.maxLength > MAX_TEXT_RESPONSE_CHARS
  ) {
    issues.maxLength = `Set a maximum length from 1 through ${MAX_TEXT_RESPONSE_CHARS}.`;
  }
  const seen = new Set<string>();
  response.answers.forEach((answer, index) => {
    const normalized = normalizedAcceptedAnswer(answer);
    if (normalized === "") {
      issues[`answers.${index}`] = "Enter an accepted answer or remove this row.";
      return;
    }
    if (Array.from(answer).length > MAX_TEXT_RESPONSE_CHARS) {
      issues[`answers.${index}`] = `Keep this answer within ${MAX_TEXT_RESPONSE_CHARS} characters.`;
      return;
    }
    if (answer.length > response.maxLength) {
      issues[`answers.${index}`] = "This answer is longer than the learner response limit.";
      return;
    }
    if (seen.has(normalized)) {
      issues[`answers.${index}`] =
        "This repeats another accepted answer after ordinary spacing and capitalization are ignored.";
      return;
    }
    seen.add(normalized);
  });
  if (response.answers.length === 0) {
    issues.answers = "Add at least one accepted answer.";
  }
  return { valid: Object.keys(issues).length === 0, issues };
}

export function setFillInAnswer(
  response: FlatQuestionFillInResponse,
  index: number,
  answer: string,
): FlatQuestionFillInResponse {
  if (!Number.isInteger(index) || index < 0 || index >= response.answers.length) return response;
  return {
    ...response,
    answers: response.answers.map((current, currentIndex) =>
      currentIndex === index ? answer : current,
    ),
  };
}

export function addFillInAnswer(response: FlatQuestionFillInResponse): FlatQuestionFillInResponse {
  return { ...response, answers: [...response.answers, "New accepted answer"] };
}

export function removeFillInAnswer(
  response: FlatQuestionFillInResponse,
  index: number,
): FlatQuestionFillInResponse {
  if (
    response.answers.length <= 1 ||
    !Number.isInteger(index) ||
    index < 0 ||
    index >= response.answers.length
  ) {
    return response;
  }
  return {
    ...response,
    answers: response.answers.filter((_answer, currentIndex) => currentIndex !== index),
  };
}

export function setFillInMatchMode(
  response: FlatQuestionFillInResponse,
  matchMode: FlatQuestionTextMatchMode,
): FlatQuestionFillInResponse {
  return { ...response, matchMode };
}

export function setFillInMaxLength(
  response: FlatQuestionFillInResponse,
  maxLength: number,
): FlatQuestionFillInResponse {
  return { ...response, maxLength };
}
