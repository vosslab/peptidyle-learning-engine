import { DecodeError } from "../../api/decoder";
import { serializeFlatQuestionSource } from "./flat_question_codec";
import type {
  FlatQuestionAttemptPolicy,
  FlatQuestionChoice,
  FlatQuestionLicense,
  FlatQuestionOutcomeFeedback,
  FlatQuestionSourceV1,
  FlatQuestionTaxonomyTerm,
  FlatQuestionTimingPolicy,
} from "./flat_question_source";

export type FlatQuestionInstructorPreview = {
  readonly revision: string;
  readonly correctChoice: string;
  readonly explanation: string | null;
};

type WorkingState = {
  readonly source: FlatQuestionSourceV1;
  readonly savedSource: FlatQuestionSourceV1;
  readonly instructorPreview: FlatQuestionInstructorPreview | null;
};

export type FlatQuestionEditorState =
  | { readonly kind: "loading" }
  | (WorkingState & { readonly kind: "ready"; readonly status: "clean" | "dirty" | "saving" })
  | { readonly kind: "conflict"; readonly localSource: FlatQuestionSourceV1 }
  | { readonly kind: "reloading"; readonly localSource: FlatQuestionSourceV1 }
  | (WorkingState & { readonly kind: "publishReview"; readonly review: string })
  | (WorkingState & { readonly kind: "publishing"; readonly review: string })
  | (WorkingState & { readonly kind: "published"; readonly reference: string })
  | {
      readonly kind: "error";
      readonly message: string;
      readonly resume: "loading" | "ready" | "publishReview";
      readonly source: FlatQuestionSourceV1 | null;
      readonly savedSource: FlatQuestionSourceV1 | null;
      readonly review: string | null;
    };

export type FlatQuestionEditorAction =
  | { readonly kind: "loaded"; readonly source: FlatQuestionSourceV1 }
  | { readonly kind: "loadFailed"; readonly message: string }
  | { readonly kind: "edit"; readonly source: FlatQuestionSourceV1 }
  | { readonly kind: "saveStarted" }
  | { readonly kind: "saveSucceeded" }
  | { readonly kind: "saveFailed"; readonly message: string }
  | { readonly kind: "saveConflict" }
  | { readonly kind: "reloadStarted" }
  | { readonly kind: "reloadSucceeded"; readonly source: FlatQuestionSourceV1 }
  | { readonly kind: "reloadFailed"; readonly message: string }
  | { readonly kind: "reviewOpened"; readonly review: string }
  | { readonly kind: "publishStarted" }
  | { readonly kind: "publishSucceeded"; readonly reference: string }
  | { readonly kind: "publishFailed"; readonly message: string }
  | { readonly kind: "instructorPreviewLoaded"; readonly preview: FlatQuestionInstructorPreview }
  | { readonly kind: "dismissError" };

export type SourceEditResult = {
  readonly source: FlatQuestionSourceV1;
  readonly changed: boolean;
  readonly error: string | null;
};

export type FlatQuestionValidationIssue = {
  readonly field: string;
  readonly message: string;
};

export type FlatQuestionValidation = {
  readonly valid: boolean;
  readonly issues: ReadonlyArray<FlatQuestionValidationIssue>;
};

export function initialFlatQuestionEditorState(): FlatQuestionEditorState {
  return { kind: "loading" };
}

function ready(
  source: FlatQuestionSourceV1,
  savedSource: FlatQuestionSourceV1,
): FlatQuestionEditorState {
  const status = sourcesEqual(source, savedSource) ? "clean" : "dirty";
  return { kind: "ready", status, source, savedSource, instructorPreview: null };
}

function editorError(
  message: string,
  resume: "loading" | "ready" | "publishReview",
  source: FlatQuestionSourceV1 | null,
  savedSource: FlatQuestionSourceV1 | null,
  review: string | null,
): FlatQuestionEditorState {
  return { kind: "error", message, resume, source, savedSource, review };
}

/** Ignores actions that cannot occur in the current state, preserving a valid workflow. */
export function reduceFlatQuestionEditor(
  state: FlatQuestionEditorState,
  action: FlatQuestionEditorAction,
): FlatQuestionEditorState {
  if (state.kind === "loading") {
    if (action.kind === "loaded") return ready(action.source, action.source);
    if (action.kind === "loadFailed")
      return editorError(action.message, "loading", null, null, null);
    return state;
  }
  if (state.kind === "conflict") {
    if (action.kind === "reloadStarted")
      return { kind: "reloading", localSource: state.localSource };
    return state;
  }
  if (state.kind === "reloading") {
    if (action.kind === "reloadSucceeded") return ready(action.source, action.source);
    if (action.kind === "reloadFailed") {
      return { kind: "conflict", localSource: state.localSource };
    }
    return state;
  }
  if (state.kind === "error") {
    if (action.kind !== "dismissError") return state;
    if (state.resume === "loading") return initialFlatQuestionEditorState();
    if (state.source === null || state.savedSource === null)
      return initialFlatQuestionEditorState();
    if (state.resume === "publishReview" && state.review !== null) {
      return {
        kind: "publishReview",
        source: state.source,
        savedSource: state.savedSource,
        instructorPreview: null,
        review: state.review,
      };
    }
    return ready(state.source, state.savedSource);
  }
  if (state.kind === "published") return state;
  if (state.kind === "ready") {
    if (action.kind === "edit" && state.status !== "saving")
      return ready(action.source, state.savedSource);
    if (action.kind === "saveStarted" && state.status === "dirty")
      return { ...state, status: "saving" };
    if (action.kind === "saveSucceeded" && state.status === "saving")
      return ready(state.source, state.source);
    if (action.kind === "saveFailed" && state.status === "saving") {
      return editorError(action.message, "ready", state.source, state.savedSource, null);
    }
    if (action.kind === "saveConflict" && state.status === "saving") {
      return { kind: "conflict", localSource: state.source };
    }
    if (action.kind === "reviewOpened" && state.status === "clean") {
      return { ...state, kind: "publishReview", review: action.review };
    }
    if (action.kind === "instructorPreviewLoaded" && state.status === "clean") {
      return { ...state, instructorPreview: action.preview };
    }
    return state;
  }
  if (state.kind === "publishReview") {
    if (action.kind === "edit") return ready(action.source, state.savedSource);
    if (action.kind === "publishStarted") return { ...state, kind: "publishing" };
    if (action.kind === "instructorPreviewLoaded")
      return { ...state, instructorPreview: action.preview };
    return state;
  }
  if (action.kind === "publishSucceeded") {
    return { ...state, kind: "published", reference: action.reference };
  }
  if (action.kind === "publishFailed") {
    return editorError(
      action.message,
      "publishReview",
      state.source,
      state.savedSource,
      state.review,
    );
  }
  return state;
}

function changed(source: FlatQuestionSourceV1): SourceEditResult {
  return { source, changed: true, error: null };
}

function refused(source: FlatQuestionSourceV1, error: string): SourceEditResult {
  return { source, changed: false, error };
}

function replaceChoice(
  source: FlatQuestionSourceV1,
  choiceId: string,
  replacement: FlatQuestionChoice,
): SourceEditResult {
  const index = source.choices.findIndex((choice) => choice.id === choiceId);
  if (index < 0) return refused(source, "That choice no longer exists.");
  const choices = source.choices.map((choice) => (choice.id === choiceId ? replacement : choice));
  return changed({ ...source, choices });
}

export function setFlatQuestionTitle(
  source: FlatQuestionSourceV1,
  title: string,
): FlatQuestionSourceV1 {
  return { ...source, title };
}

export function setFlatQuestionPrompt(
  source: FlatQuestionSourceV1,
  prompt: string,
): FlatQuestionSourceV1 {
  return { ...source, prompt };
}

export function setFlatQuestionPoints(
  source: FlatQuestionSourceV1,
  points: number,
): FlatQuestionSourceV1 {
  return { ...source, points };
}

export function setChoiceText(
  source: FlatQuestionSourceV1,
  choiceId: string,
  text: string,
): SourceEditResult {
  const current = source.choices.find((choice) => choice.id === choiceId);
  return current === undefined
    ? refused(source, "That choice no longer exists.")
    : replaceChoice(source, choiceId, { ...current, text });
}

export function setChoiceFeedback(
  source: FlatQuestionSourceV1,
  choiceId: string,
  feedback: string | null,
): SourceEditResult {
  const current = source.choices.find((choice) => choice.id === choiceId);
  return current === undefined
    ? refused(source, "That choice no longer exists.")
    : replaceChoice(source, choiceId, { ...current, feedback });
}

export function setCorrectChoice(source: FlatQuestionSourceV1, choiceId: string): SourceEditResult {
  if (!source.choices.some((choice) => choice.id === choiceId)) {
    return refused(source, "Choose one of the listed answers.");
  }
  return changed({ ...source, correctChoice: choiceId });
}

export function addChoice(source: FlatQuestionSourceV1): SourceEditResult {
  if (source.choices.length >= 100)
    return refused(source, "A question can have at most 100 choices.");
  const id = nextChoiceId(source.choices);
  const choice: FlatQuestionChoice = { id, text: "New choice", feedback: null };
  return changed({ ...source, choices: [...source.choices, choice] });
}

export function removeChoice(source: FlatQuestionSourceV1, choiceId: string): SourceEditResult {
  if (source.choices.length <= 2)
    return refused(source, "A single-choice question needs at least two choices.");
  const index = source.choices.findIndex((choice) => choice.id === choiceId);
  if (index < 0) return refused(source, "That choice no longer exists.");
  const choices = source.choices.filter((choice) => choice.id !== choiceId);
  const replacement = choices[Math.min(index, choices.length - 1)];
  if (replacement === undefined) return refused(source, "A replacement answer is required.");
  const correctChoice = source.correctChoice === choiceId ? replacement.id : source.correctChoice;
  return changed({ ...source, choices, correctChoice });
}

export function reorderChoices(
  source: FlatQuestionSourceV1,
  orderedIds: ReadonlyArray<string>,
): SourceEditResult {
  if (
    orderedIds.length !== source.choices.length ||
    new Set(orderedIds).size !== orderedIds.length
  ) {
    return refused(source, "Use every choice exactly once when reordering.");
  }
  const byId = new Map(source.choices.map((choice) => [choice.id, choice]));
  const choices: FlatQuestionChoice[] = [];
  for (const id of orderedIds) {
    const choice = byId.get(id);
    if (choice === undefined) {
      return refused(source, "Use every choice exactly once when reordering.");
    }
    choices.push(choice);
  }
  return changed({ ...source, choices });
}

export function renameChoiceId(
  source: FlatQuestionSourceV1,
  previousId: string,
  nextId: string,
): SourceEditResult {
  if (!/^[a-z][a-z0-9_-]*$/u.test(nextId)) {
    return refused(source, "Choice IDs use lowercase letters, digits, underscores, and hyphens.");
  }
  if (source.choices.some((choice) => choice.id === nextId && choice.id !== previousId)) {
    return refused(source, "Each choice needs a unique ID.");
  }
  const current = source.choices.find((choice) => choice.id === previousId);
  if (current === undefined) return refused(source, "That choice no longer exists.");
  const choices = source.choices.map((choice) =>
    choice.id === previousId ? { ...choice, id: nextId } : choice,
  );
  const correctChoice = source.correctChoice === previousId ? nextId : source.correctChoice;
  return changed({ ...source, choices, correctChoice });
}

export function setOutcomeFeedback(
  source: FlatQuestionSourceV1,
  feedback: FlatQuestionOutcomeFeedback,
): FlatQuestionSourceV1 {
  return { ...source, feedback };
}

export function setAttemptPolicy(
  source: FlatQuestionSourceV1,
  attemptPolicy: FlatQuestionAttemptPolicy,
): FlatQuestionSourceV1 {
  return { ...source, attemptPolicy };
}

export function setTimingPolicy(
  source: FlatQuestionSourceV1,
  timingPolicy: FlatQuestionTimingPolicy,
): FlatQuestionSourceV1 {
  return { ...source, timingPolicy };
}

export function setTags(
  source: FlatQuestionSourceV1,
  tags: ReadonlyArray<string>,
): FlatQuestionSourceV1 {
  return { ...source, tags: [...tags] };
}

export function setTaxonomy(
  source: FlatQuestionSourceV1,
  taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>,
): FlatQuestionSourceV1 {
  return { ...source, taxonomy: [...taxonomy] };
}

export function setLicense(
  source: FlatQuestionSourceV1,
  license: FlatQuestionLicense,
): FlatQuestionSourceV1 {
  return { ...source, license };
}

export function setLanguage(source: FlatQuestionSourceV1, language: string): FlatQuestionSourceV1 {
  return { ...source, language };
}

/** Uses the canonical codec and turns its structural result into safe field-level author guidance. */
export function validateFlatQuestionSource(source: FlatQuestionSourceV1): FlatQuestionValidation {
  try {
    serializeFlatQuestionSource(source);
    return { valid: true, issues: [] };
  } catch (error: unknown) {
    const path =
      error instanceof DecodeError ? (error.message.split(" must ", 1)[0] ?? "source") : "source";
    const field = path.replace(/^source\.?/u, "") || "question";
    return { valid: false, issues: [{ field, message: validationMessage(field) }] };
  }
}

/** Compares canonical bytes when valid, with deterministic source shape fallback while typing. */
export function sourcesEqual(left: FlatQuestionSourceV1, right: FlatQuestionSourceV1): boolean {
  return stableSourceText(left) === stableSourceText(right);
}

function stableSourceText(source: FlatQuestionSourceV1): string {
  try {
    return serializeFlatQuestionSource(source);
  } catch {
    return JSON.stringify(source);
  }
}

function nextChoiceId(choices: ReadonlyArray<FlatQuestionChoice>): string {
  const ids = new Set(choices.map((choice) => choice.id));
  let index = 1;
  while (ids.has(`choice_${index}`)) index += 1;
  return `choice_${index}`;
}

function validationMessage(field: string): string {
  if (field.startsWith("choices")) return "Check the choices and select one correct answer.";
  if (field.startsWith("title")) return "Add a short question title.";
  if (field.startsWith("prompt")) return "Add the learner-facing question prompt.";
  if (field.startsWith("points")) return "Points must be a nonnegative number.";
  if (field.startsWith("timingPolicy")) return "Check the timing policy values.";
  return "Check the question details before saving.";
}
