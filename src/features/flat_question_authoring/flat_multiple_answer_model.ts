// flat_multiple_answer_model.ts - identity-preserving private edits for multiple-answer choices.

import type {
  FlatQuestionChoice,
  FlatQuestionMultipleAnswerResponse,
} from "./flat_question_source";

export const MINIMUM_MULTIPLE_ANSWER_CHOICES = 2;
export const MAXIMUM_MULTIPLE_ANSWER_CHOICES = 100;

export type MultipleAnswerEditResult = {
  readonly response: FlatQuestionMultipleAnswerResponse;
  readonly changed: boolean;
  readonly error?: string;
};

export type MultipleAnswerValidation = Readonly<Record<string, string | undefined>>;

function changed(response: FlatQuestionMultipleAnswerResponse): MultipleAnswerEditResult {
  return { response, changed: true };
}

function refused(
  response: FlatQuestionMultipleAnswerResponse,
  error: string,
): MultipleAnswerEditResult {
  return { response, changed: false, error };
}

function choiceAt(
  response: FlatQuestionMultipleAnswerResponse,
  choiceId: string,
): FlatQuestionChoice | undefined {
  return response.choices.find((choice) => choice.id === choiceId);
}

function nextChoiceId(choices: ReadonlyArray<FlatQuestionChoice>): string {
  let suffix = 1;
  while (choices.some((choice) => choice.id === `choice_${suffix}`)) suffix += 1;
  return `choice_${suffix}`;
}

/** Edits text without changing the semantic identifier used by keys and learner responses. */
export function setMultipleAnswerChoiceText(
  response: FlatQuestionMultipleAnswerResponse,
  choiceId: string,
  text: string,
): MultipleAnswerEditResult {
  const current = choiceAt(response, choiceId);
  if (current === undefined) return refused(response, "That choice no longer exists.");
  const choices = response.choices.map((choice) =>
    choice.id === choiceId ? { ...current, text } : choice,
  );
  return changed({ ...response, choices });
}

export function setMultipleAnswerChoiceFeedback(
  response: FlatQuestionMultipleAnswerResponse,
  choiceId: string,
  feedback: string | null,
): MultipleAnswerEditResult {
  const current = choiceAt(response, choiceId);
  if (current === undefined) return refused(response, "That choice no longer exists.");
  const choices = response.choices.map((choice) =>
    choice.id === choiceId ? { ...current, feedback } : choice,
  );
  return changed({ ...response, choices });
}

/** Adds a new wrong answer; the author marks it correct explicitly when appropriate. */
export function addMultipleAnswerChoice(
  response: FlatQuestionMultipleAnswerResponse,
): MultipleAnswerEditResult {
  if (response.choices.length >= MAXIMUM_MULTIPLE_ANSWER_CHOICES) {
    return refused(response, "A question can have at most 100 choices.");
  }
  const choice: FlatQuestionChoice = {
    id: nextChoiceId(response.choices),
    text: "New choice",
    feedback: null,
  };
  const choices = [...response.choices, choice];
  return changed({ ...response, choices });
}

/** Removing a correct choice deliberately removes only that choice from the private answer set. */
export function removeMultipleAnswerChoice(
  response: FlatQuestionMultipleAnswerResponse,
  choiceId: string,
): MultipleAnswerEditResult {
  if (response.choices.length <= MINIMUM_MULTIPLE_ANSWER_CHOICES) {
    return refused(response, "A multiple-answer question needs at least two choices.");
  }
  if (choiceAt(response, choiceId) === undefined) {
    return refused(response, "That choice no longer exists.");
  }
  const choices = response.choices.filter((choice) => choice.id !== choiceId);
  const correctChoices = response.correctChoices.filter((id) => id !== choiceId);
  return changed({ ...response, choices, correctChoices });
}

/** Toggle actions never duplicate an ID; the final selection may be empty so the inline validator can explain it. */
export function setMultipleAnswerCorrect(
  response: FlatQuestionMultipleAnswerResponse,
  choiceId: string,
  correct: boolean,
): MultipleAnswerEditResult {
  if (choiceAt(response, choiceId) === undefined) {
    return refused(response, "Choose one of the listed answers.");
  }
  const alreadyCorrect = response.correctChoices.includes(choiceId);
  if (alreadyCorrect === correct) return { response, changed: false };
  const correctChoices = correct
    ? [...response.correctChoices, choiceId]
    : response.correctChoices.filter((id) => id !== choiceId);
  return changed({ ...response, correctChoices });
}

/** Reordering changes reading order only. Semantic IDs and the private answer set are retained. */
export function moveMultipleAnswerChoice(
  response: FlatQuestionMultipleAnswerResponse,
  choiceId: string,
  direction: "earlier" | "later",
): MultipleAnswerEditResult {
  const index = response.choices.findIndex((choice) => choice.id === choiceId);
  if (index < 0) return refused(response, "That choice no longer exists.");
  const destination = direction === "earlier" ? index - 1 : index + 1;
  if (destination < 0 || destination >= response.choices.length) {
    return refused(response, "That choice is already at the requested position.");
  }
  const choices = [...response.choices];
  const current = choices[index];
  const target = choices[destination];
  if (current === undefined || target === undefined)
    return refused(response, "Unable to move that choice.");
  choices[index] = target;
  choices[destination] = current;
  return changed({ ...response, choices });
}

/** Produces field-addressable, recovery-oriented messages for the protected authoring panel. */
export function validateMultipleAnswerResponse(
  response: FlatQuestionMultipleAnswerResponse,
): MultipleAnswerValidation {
  const errors: Record<string, string> = {};
  if (response.choices.length < MINIMUM_MULTIPLE_ANSWER_CHOICES) {
    errors.choices = "Add another choice. Multiple-answer questions need at least two choices.";
  }
  if (response.correctChoices.length === 0) {
    errors.correctChoices = "Mark at least one choice as a correct answer before saving.";
  }
  const choiceIds = new Set(response.choices.map((choice) => choice.id));
  if (response.correctChoices.some((id) => !choiceIds.has(id))) {
    errors.correctChoices = "Choose correct answers from the current choice list.";
  }
  if (new Set(response.correctChoices).size !== response.correctChoices.length) {
    errors.correctChoices = "Mark each correct answer only once.";
  }
  return errors;
}
