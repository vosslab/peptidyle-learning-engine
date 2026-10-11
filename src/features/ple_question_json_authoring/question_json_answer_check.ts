// Private instructor answer-check mapping from Native Question source.

import type { PleQuestionJsonInstructorAnswerCheck } from "./question_json_preview";
import type { PleQuestionJsonDocument, PleQuestionJsonOrderingItem } from "./question_json_source";

export function questionJsonAnswerCheck(
  source: PleQuestionJsonDocument,
): PleQuestionJsonInstructorAnswerCheck | null {
  const response = source.response;
  if (response.kind === "multipleAnswer") {
    const correctChoiceTexts = response.choices
      .filter((choice) => response.correctChoices.includes(choice.id))
      .map((choice) => choice.text);
    return correctChoiceTexts.length === 0 ? null : { kind: "multipleAnswer", correctChoiceTexts };
  }
  if (response.kind === "fillIn") {
    return { kind: "fillIn", answers: response.answers, matchMode: response.matchMode };
  }
  if (response.kind === "multiFillIn") {
    return {
      kind: "multiFillIn",
      blanks: response.blanks.map((blank) => ({ label: blank.label, answers: blank.answers })),
    };
  }
  if (response.kind === "numeric") {
    return {
      kind: "numeric",
      answer: response.answer,
      tolerance: response.tolerance,
      unit: response.unit,
    };
  }
  if (response.kind === "ordering") {
    const items = response.correctOrder.map((id) => response.items.find((item) => item.id === id));
    if (items.some((item) => item === undefined)) return null;
    return {
      kind: "ordering",
      items: items.filter((item): item is PleQuestionJsonOrderingItem => item !== undefined),
    };
  }
  if (response.kind === "matching") {
    const prompts = new Map(response.prompts.map((item) => [item.id, item.text]));
    const choices = new Map(response.choices.map((item) => [item.id, item.text]));
    const pairs = response.matches.map((pair) => {
      const prompt = prompts.get(pair.prompt);
      const choice = choices.get(pair.choice);
      return prompt === undefined || choice === undefined ? null : ([prompt, choice] as const);
    });
    if (pairs.some((pair) => pair === null)) return null;
    return {
      kind: "matching",
      pairs: pairs.filter((pair): pair is readonly [string, string] => pair !== null),
    };
  }
  if (response.kind !== "singleChoice") return null;
  const correct = response.choices.find((choice) => choice.id === response.correctChoice);
  if (correct === undefined) return null;
  return {
    kind: "singleChoice",
    correctChoiceText: correct.text,
    correctFeedback: source.feedback.correct,
    incorrectFeedback: source.feedback.incorrect,
  };
}
