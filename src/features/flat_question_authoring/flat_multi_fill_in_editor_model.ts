// flat_multi_fill_in_editor_model.ts - identity-safe edits for MULTI-FIB authoring.

import type {
  FlatQuestionBlank,
  FlatQuestionSourceV2,
  FlatQuestionTextMatchMode,
} from "./flat_question_source";

const MAX_BLANKS = 50;
const MAX_ANSWER_LENGTH = 16_384;

export type FlatMultiFillInEditResult = {
  readonly source: FlatQuestionSourceV2;
  readonly changed: boolean;
  readonly error: string | null;
  readonly focusId: string | null;
  readonly status: string | null;
};

function changed(
  source: FlatQuestionSourceV2,
  focusId: string | null,
  status: string | null,
): FlatMultiFillInEditResult {
  return { source, changed: true, error: null, focusId, status };
}

function refused(source: FlatQuestionSourceV2, error: string): FlatMultiFillInEditResult {
  return { source, changed: false, error, focusId: null, status: null };
}

function multiFillResponse(
  source: FlatQuestionSourceV2,
): Extract<FlatQuestionSourceV2["response"], { readonly kind: "multiFillIn" }> | null {
  return source.response.kind === "multiFillIn" ? source.response : null;
}

function replaceBlank(
  source: FlatQuestionSourceV2,
  blankId: string,
  replacement: FlatQuestionBlank,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  if (response === null) return refused(source, "Choose multiple fill in before editing blanks.");
  if (!response.blanks.some((blank) => blank.id === blankId)) {
    return refused(source, "That blank no longer exists.");
  }
  const blanks = response.blanks.map((blank) => (blank.id === blankId ? replacement : blank));
  return changed(
    { ...source, response: { ...response, blanks } },
    blankId,
    `Updated ${replacement.label || "blank"}.`,
  );
}

/** Adds a complete blank, so a transient authoring state never loses answer semantics. */
export function addMultiFillBlank(source: FlatQuestionSourceV2): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  if (response === null) return refused(source, "Choose multiple fill in before adding blanks.");
  if (response.blanks.length >= MAX_BLANKS) {
    return refused(source, `A question can have at most ${MAX_BLANKS} blanks.`);
  }
  const id = nextBlankId(response.blanks);
  const blank: FlatQuestionBlank = {
    id,
    label: "New blank",
    answers: ["Accepted answer"],
    matchMode: "caseInsensitive",
    maxLength: 256,
  };
  const blanks = [...response.blanks, blank];
  return changed(
    { ...source, response: { ...response, blanks } },
    id,
    "Added a blank with one accepted answer.",
  );
}

export function removeMultiFillBlank(
  source: FlatQuestionSourceV2,
  blankId: string,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  if (response === null) return refused(source, "Choose multiple fill in before editing blanks.");
  if (response.blanks.length <= 1) {
    return refused(source, "A multiple-fill question needs at least one blank.");
  }
  const index = response.blanks.findIndex((blank) => blank.id === blankId);
  if (index < 0) return refused(source, "That blank no longer exists.");
  const blanks = response.blanks.filter((blank) => blank.id !== blankId);
  const focusBlank = blanks[Math.min(index, blanks.length - 1)];
  if (focusBlank === undefined)
    return refused(source, "Choose a remaining blank before removing this one.");
  return changed(
    { ...source, response: { ...response, blanks } },
    focusBlank.id,
    `Removed blank ${index + 1}; its accepted answers were removed with it.`,
  );
}

/** Reorders whole blank records so every accepted-answer relation travels with its stable blank ID. */
export function reorderMultiFillBlanks(
  source: FlatQuestionSourceV2,
  orderedIds: ReadonlyArray<string>,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  if (response === null)
    return refused(source, "Choose multiple fill in before reordering blanks.");
  if (
    orderedIds.length !== response.blanks.length ||
    new Set(orderedIds).size !== orderedIds.length
  ) {
    return refused(source, "Use every blank exactly once when reordering.");
  }
  const blanksById = new Map(response.blanks.map((blank) => [blank.id, blank]));
  const blanks: FlatQuestionBlank[] = [];
  for (const id of orderedIds) {
    const blank = blanksById.get(id);
    if (blank === undefined)
      return refused(source, "Use every blank exactly once when reordering.");
    blanks.push(blank);
  }
  return changed(
    { ...source, response: { ...response, blanks } },
    orderedIds[0] ?? null,
    "Reordered blanks; accepted answers remain attached to their blank.",
  );
}

export function setMultiFillBlankLabel(
  source: FlatQuestionSourceV2,
  blankId: string,
  label: string,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  const blank = response?.blanks.find((candidate) => candidate.id === blankId);
  if (blank === undefined) return refused(source, "That blank no longer exists.");
  return replaceBlank(source, blankId, { ...blank, label });
}

export function setMultiFillBlankMatchMode(
  source: FlatQuestionSourceV2,
  blankId: string,
  matchMode: FlatQuestionTextMatchMode,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  const blank = response?.blanks.find((candidate) => candidate.id === blankId);
  if (blank === undefined) return refused(source, "That blank no longer exists.");
  return replaceBlank(source, blankId, { ...blank, matchMode });
}

export function setMultiFillBlankMaxLength(
  source: FlatQuestionSourceV2,
  blankId: string,
  maxLength: number,
): FlatMultiFillInEditResult {
  if (!Number.isInteger(maxLength) || maxLength < 1 || maxLength > MAX_ANSWER_LENGTH) {
    return refused(source, `Maximum length must be an integer from 1 to ${MAX_ANSWER_LENGTH}.`);
  }
  const response = multiFillResponse(source);
  const blank = response?.blanks.find((candidate) => candidate.id === blankId);
  if (blank === undefined) return refused(source, "That blank no longer exists.");
  if (blank.answers.some((answer) => answer.length > maxLength)) {
    return refused(source, "Increase maximum length or shorten each accepted answer first.");
  }
  return replaceBlank(source, blankId, { ...blank, maxLength });
}

export function setMultiFillBlankAnswer(
  source: FlatQuestionSourceV2,
  blankId: string,
  answerIndex: number,
  answer: string,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  const blank = response?.blanks.find((candidate) => candidate.id === blankId);
  if (blank === undefined) return refused(source, "That blank no longer exists.");
  if (answerIndex < 0 || answerIndex >= blank.answers.length) {
    return refused(source, "That accepted answer no longer exists.");
  }
  if (answer.length === 0 || answer.length > blank.maxLength) {
    return refused(source, `Use an accepted answer from 1 to ${blank.maxLength} characters.`);
  }
  if (blank.answers.some((current, index) => index !== answerIndex && current === answer)) {
    return refused(source, "Each accepted answer for a blank must be unique.");
  }
  const answers = blank.answers.map((current, index) => (index === answerIndex ? answer : current));
  return replaceBlank(source, blankId, { ...blank, answers });
}

export function addMultiFillBlankAnswer(
  source: FlatQuestionSourceV2,
  blankId: string,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  const blank = response?.blanks.find((candidate) => candidate.id === blankId);
  if (blank === undefined) return refused(source, "That blank no longer exists.");
  const answer = nextAcceptedAnswer(blank.answers, blank.maxLength);
  const answers = [...blank.answers, answer];
  return replaceBlank(source, blankId, { ...blank, answers });
}

export function removeMultiFillBlankAnswer(
  source: FlatQuestionSourceV2,
  blankId: string,
  answerIndex: number,
): FlatMultiFillInEditResult {
  const response = multiFillResponse(source);
  const blank = response?.blanks.find((candidate) => candidate.id === blankId);
  if (blank === undefined) return refused(source, "That blank no longer exists.");
  if (blank.answers.length <= 1) {
    return refused(source, "Each blank needs at least one accepted answer.");
  }
  if (answerIndex < 0 || answerIndex >= blank.answers.length) {
    return refused(source, "That accepted answer no longer exists.");
  }
  const answers = blank.answers.filter((_answer, index) => index !== answerIndex);
  return replaceBlank(source, blankId, { ...blank, answers });
}

function nextBlankId(blanks: ReadonlyArray<FlatQuestionBlank>): string {
  const ids = new Set(blanks.map((blank) => blank.id));
  let index = 1;
  while (ids.has(`blank_${index}`)) index += 1;
  return `blank_${index}`;
}

function nextAcceptedAnswer(answers: ReadonlyArray<string>, maxLength: number): string {
  const candidate = "Alternative answer".slice(0, maxLength);
  if (!answers.includes(candidate)) return candidate;
  let index = 2;
  while (answers.includes(`Alternative ${index}`.slice(0, maxLength))) index += 1;
  return `Alternative ${index}`.slice(0, maxLength);
}
