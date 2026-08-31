import { DecodeError } from "../../api/decoder";
import { serializeFlatQuestionSource } from "./flat_question_codec";
import type {
  FlatQuestionAttemptPolicy,
  FlatQuestionChoice,
  FlatQuestionItem,
  FlatQuestionLicense,
  FlatQuestionOutcomeFeedback,
  FlatQuestionSourceV2,
  FlatQuestionTaxonomyTerm,
  FlatQuestionTimingPolicy,
} from "./flat_question_source";

const DEFAULT_CHOICES: ReadonlyArray<FlatQuestionChoice> = [
  { id: "choice_a", text: "First choice", feedback: null },
  { id: "choice_b", text: "Second choice", feedback: null },
];

export type FlatQuestionInstructorPreview = {
  readonly revision: string;
  readonly correctChoice: string;
  readonly explanation: string | null;
};

type WorkingState = {
  readonly source: FlatQuestionSourceV2;
  readonly savedSource: FlatQuestionSourceV2;
  readonly instructorPreview: FlatQuestionInstructorPreview | null;
};

export type FlatQuestionEditorState =
  | { readonly kind: "loading" }
  | (WorkingState & { readonly kind: "ready"; readonly status: "clean" | "dirty" | "saving" })
  | { readonly kind: "conflict"; readonly localSource: FlatQuestionSourceV2 }
  | { readonly kind: "reloading"; readonly localSource: FlatQuestionSourceV2 }
  | (WorkingState & { readonly kind: "publishReview"; readonly review: string })
  | (WorkingState & { readonly kind: "publishing"; readonly review: string })
  | (WorkingState & { readonly kind: "published"; readonly reference: string })
  | {
      readonly kind: "error";
      readonly message: string;
      readonly resume: "loading" | "ready" | "publishReview";
      readonly source: FlatQuestionSourceV2 | null;
      readonly savedSource: FlatQuestionSourceV2 | null;
      readonly review: string | null;
    };

export type FlatQuestionEditorAction =
  | { readonly kind: "loaded"; readonly source: FlatQuestionSourceV2 }
  | { readonly kind: "loadFailed"; readonly message: string }
  | { readonly kind: "edit"; readonly source: FlatQuestionSourceV2 }
  | { readonly kind: "saveStarted" }
  | { readonly kind: "saveSucceeded" }
  | { readonly kind: "saveFailed"; readonly message: string }
  | { readonly kind: "saveConflict" }
  | { readonly kind: "reloadStarted" }
  | { readonly kind: "reloadSucceeded"; readonly source: FlatQuestionSourceV2 }
  | { readonly kind: "reloadFailed"; readonly message: string }
  | { readonly kind: "reviewOpened"; readonly review: string }
  | { readonly kind: "publishStarted" }
  | { readonly kind: "publishSucceeded"; readonly reference: string }
  | { readonly kind: "publishFailed"; readonly message: string }
  | { readonly kind: "instructorPreviewLoaded"; readonly preview: FlatQuestionInstructorPreview }
  | { readonly kind: "dismissError" };

export type SourceEditResult = {
  readonly source: FlatQuestionSourceV2;
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
  source: FlatQuestionSourceV2,
  savedSource: FlatQuestionSourceV2,
): FlatQuestionEditorState {
  const status = sourcesEqual(source, savedSource) ? "clean" : "dirty";
  return { kind: "ready", status, source, savedSource, instructorPreview: null };
}

function editorError(
  message: string,
  resume: "loading" | "ready" | "publishReview",
  source: FlatQuestionSourceV2 | null,
  savedSource: FlatQuestionSourceV2 | null,
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

function changed(source: FlatQuestionSourceV2): SourceEditResult {
  return { source, changed: true, error: null };
}

function refused(source: FlatQuestionSourceV2, error: string): SourceEditResult {
  return { source, changed: false, error };
}

function replaceChoice(
  source: FlatQuestionSourceV2,
  choiceId: string,
  replacement: FlatQuestionChoice,
): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before editing choices.");
  }
  const index = source.response.choices.findIndex((choice) => choice.id === choiceId);
  if (index < 0) return refused(source, "That choice no longer exists.");
  const choices = source.response.choices.map((choice) =>
    choice.id === choiceId ? replacement : choice,
  );
  return changed({ ...source, response: { ...source.response, choices } });
}

export function setFlatQuestionTitle(
  source: FlatQuestionSourceV2,
  title: string,
): FlatQuestionSourceV2 {
  return { ...source, title };
}

export function setFlatQuestionPrompt(
  source: FlatQuestionSourceV2,
  prompt: string,
): FlatQuestionSourceV2 {
  return { ...source, prompt };
}

export function setMatchingItemText(
  source: FlatQuestionSourceV2,
  side: "prompts" | "choices",
  itemId: string,
  text: string,
): SourceEditResult {
  if (source.response.kind !== "matching")
    return refused(source, "Choose matching before editing pairs.");
  const items = source.response[side];
  if (!items.some((item) => item.id === itemId))
    return refused(source, "That matching item no longer exists.");
  const nextItems = items.map((item) => (item.id === itemId ? { ...item, text } : item));
  return changed({ ...source, response: { ...source.response, [side]: nextItems } });
}

export function setMatchingPair(
  source: FlatQuestionSourceV2,
  promptId: string,
  choiceId: string,
): SourceEditResult {
  if (source.response.kind !== "matching")
    return refused(source, "Choose matching before editing pairs.");
  const promptKnown = source.response.prompts.some((item) => item.id === promptId);
  const choiceKnown = source.response.choices.some((item) => item.id === choiceId);
  if (!promptKnown || !choiceKnown)
    return refused(source, "Pair an available prompt with an available choice.");
  if (
    source.response.matches.some((pair) => pair.prompt !== promptId && pair.choice === choiceId)
  ) {
    return refused(source, "Each matching choice is used once.");
  }
  const matches = source.response.matches.map((pair) =>
    pair.prompt === promptId ? { prompt: promptId, choice: choiceId } : pair,
  );
  return changed({ ...source, response: { ...source.response, matches } });
}

const MINIMUM_MATCHING_PAIRS = 2;
const MAXIMUM_MATCHING_PAIRS = 100;

/** Adds one complete semantic pair so the private answer map stays total while the author edits. */
export function addMatchingPair(source: FlatQuestionSourceV2): SourceEditResult {
  if (source.response.kind !== "matching")
    return refused(source, "Choose matching before adding pairs.");
  if (source.response.prompts.length >= MAXIMUM_MATCHING_PAIRS) {
    return refused(source, "A matching question can have at most 100 pairs.");
  }
  const promptId = nextMatchingId(source.response.prompts, "prompt");
  const choiceId = nextMatchingId(source.response.choices, "choice");
  const prompts = [...source.response.prompts, { id: promptId, text: "New prompt" }];
  const choices = [...source.response.choices, { id: choiceId, text: "New choice" }];
  const matches = [...source.response.matches, { prompt: promptId, choice: choiceId }];
  return changed({ ...source, response: { ...source.response, prompts, choices, matches } });
}

/** Removes the prompt, its paired choice, and exactly their private relation as one atomic edit. */
export function removeMatchingPair(
  source: FlatQuestionSourceV2,
  promptId: string,
): SourceEditResult {
  if (source.response.kind !== "matching")
    return refused(source, "Choose matching before removing pairs.");
  if (source.response.prompts.length <= MINIMUM_MATCHING_PAIRS) {
    return refused(source, "A matching question needs at least two pairs.");
  }
  const pair = source.response.matches.find((candidate) => candidate.prompt === promptId);
  if (pair === undefined)
    return refused(source, "That matching prompt no longer has a paired choice.");
  const prompts = source.response.prompts.filter((item) => item.id !== promptId);
  const choices = source.response.choices.filter((item) => item.id !== pair.choice);
  const matches = source.response.matches.filter((candidate) => candidate.prompt !== promptId);
  return changed({ ...source, response: { ...source.response, prompts, choices, matches } });
}

/** Reordering changes only reading order; semantic IDs and the private pairing map are retained. */
export function reorderMatchingItems(
  source: FlatQuestionSourceV2,
  side: "prompts" | "choices",
  orderedIds: ReadonlyArray<string>,
): SourceEditResult {
  if (source.response.kind !== "matching")
    return refused(source, "Choose matching before reordering pairs.");
  const items = source.response[side];
  if (orderedIds.length !== items.length || new Set(orderedIds).size !== orderedIds.length) {
    return refused(source, "Use every matching item exactly once when reordering.");
  }
  const byId = new Map(items.map((item) => [item.id, item]));
  const reordered: FlatQuestionItem[] = [];
  for (const id of orderedIds) {
    const item = byId.get(id);
    if (item === undefined)
      return refused(source, "Use every matching item exactly once when reordering.");
    reordered.push(item);
  }
  return changed({ ...source, response: { ...source.response, [side]: reordered } });
}

function nextMatchingId(
  items: ReadonlyArray<FlatQuestionItem>,
  prefix: "prompt" | "choice",
): string {
  let suffix = 1;
  while (items.some((item) => item.id === `${prefix}_${suffix}`)) suffix += 1;
  return `${prefix}_${suffix}`;
}

/**
 * Converts ordinary text-response families to complete defaults. HOTSPOT intentionally has no
 * synthetic default: its source begins only after the private image picker returns a descriptor.
 */
export function setFlatQuestionResponseKind(
  source: FlatQuestionSourceV2,
  kind: Exclude<FlatQuestionSourceV2["response"]["kind"], "hotspot">,
): FlatQuestionSourceV2 {
  if (source.response.kind === kind) return source;
  return { ...source, response: defaultResponse(kind) };
}

function defaultResponse(
  kind: Exclude<FlatQuestionSourceV2["response"]["kind"], "hotspot">,
): FlatQuestionSourceV2["response"] {
  switch (kind) {
    case "singleChoice":
      return { kind, choices: DEFAULT_CHOICES, correctChoice: "choice_a" };
    case "multipleAnswer":
      return { kind, choices: DEFAULT_CHOICES, correctChoices: ["choice_a"] };
    case "fillIn":
      return { kind, answers: ["Accepted answer"], matchMode: "caseInsensitive", maxLength: 256 };
    case "multiFillIn":
      return {
        kind,
        blanks: [
          {
            id: "blank_a",
            label: "First blank",
            answers: ["Accepted answer"],
            matchMode: "caseInsensitive",
            maxLength: 256,
          },
        ],
      };
    case "numeric":
      return { kind, answer: 0, tolerance: { kind: "exact" }, unit: null };
    case "matching":
      return {
        kind,
        prompts: [
          { id: "prompt_a", text: "First prompt" },
          { id: "prompt_b", text: "Second prompt" },
        ],
        choices: [
          { id: "choice_a", text: "First choice" },
          { id: "choice_b", text: "Second choice" },
        ],
        matches: [
          { prompt: "prompt_a", choice: "choice_a" },
          { prompt: "prompt_b", choice: "choice_b" },
        ],
      };
    case "ordering":
      return {
        kind,
        items: [
          { id: "item_a", text: "First item" },
          { id: "item_b", text: "Second item" },
          { id: "item_c", text: "Third item" },
        ],
        correctOrder: ["item_a", "item_b", "item_c"],
      };
  }
}

export function setFlatQuestionPoints(
  source: FlatQuestionSourceV2,
  points: number,
): FlatQuestionSourceV2 {
  return { ...source, points };
}

export function setChoiceText(
  source: FlatQuestionSourceV2,
  choiceId: string,
  text: string,
): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before editing choices.");
  }
  const current = source.response.choices.find((choice) => choice.id === choiceId);
  return current === undefined
    ? refused(source, "That choice no longer exists.")
    : replaceChoice(source, choiceId, { ...current, text });
}

export function setChoiceFeedback(
  source: FlatQuestionSourceV2,
  choiceId: string,
  feedback: string | null,
): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before editing choices.");
  }
  const current = source.response.choices.find((choice) => choice.id === choiceId);
  return current === undefined
    ? refused(source, "That choice no longer exists.")
    : replaceChoice(source, choiceId, { ...current, feedback });
}

export function setCorrectChoice(source: FlatQuestionSourceV2, choiceId: string): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before setting its answer.");
  }
  if (!source.response.choices.some((choice) => choice.id === choiceId)) {
    return refused(source, "Choose one of the listed answers.");
  }
  return changed({ ...source, response: { ...source.response, correctChoice: choiceId } });
}

export function addChoice(source: FlatQuestionSourceV2): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before adding choices.");
  }
  if (source.response.choices.length >= 100)
    return refused(source, "A question can have at most 100 choices.");
  const id = nextChoiceId(source.response.choices);
  const choice: FlatQuestionChoice = { id, text: "New choice", feedback: null };
  const choices = [...source.response.choices, choice];
  return changed({ ...source, response: { ...source.response, choices } });
}

export function removeChoice(source: FlatQuestionSourceV2, choiceId: string): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before editing choices.");
  }
  if (source.response.choices.length <= 2)
    return refused(source, "A single-choice question needs at least two choices.");
  const index = source.response.choices.findIndex((choice) => choice.id === choiceId);
  if (index < 0) return refused(source, "That choice no longer exists.");
  const choices = source.response.choices.filter((choice) => choice.id !== choiceId);
  const replacement = choices[Math.min(index, choices.length - 1)];
  if (replacement === undefined) return refused(source, "A replacement answer is required.");
  const correctChoice =
    source.response.correctChoice === choiceId ? replacement.id : source.response.correctChoice;
  return changed({ ...source, response: { ...source.response, choices, correctChoice } });
}

export function reorderChoices(
  source: FlatQuestionSourceV2,
  orderedIds: ReadonlyArray<string>,
): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before reordering choices.");
  }
  if (
    orderedIds.length !== source.response.choices.length ||
    new Set(orderedIds).size !== orderedIds.length
  ) {
    return refused(source, "Use every choice exactly once when reordering.");
  }
  const byId = new Map(source.response.choices.map((choice) => [choice.id, choice]));
  const choices: FlatQuestionChoice[] = [];
  for (const id of orderedIds) {
    const choice = byId.get(id);
    if (choice === undefined) {
      return refused(source, "Use every choice exactly once when reordering.");
    }
    choices.push(choice);
  }
  return changed({ ...source, response: { ...source.response, choices } });
}

export function renameChoiceId(
  source: FlatQuestionSourceV2,
  previousId: string,
  nextId: string,
): SourceEditResult {
  if (source.response.kind !== "singleChoice") {
    return refused(source, "Choose single choice before renaming choices.");
  }
  if (!/^[a-z][a-z0-9_-]*$/u.test(nextId)) {
    return refused(source, "Choice IDs use lowercase letters, digits, underscores, and hyphens.");
  }
  if (source.response.choices.some((choice) => choice.id === nextId && choice.id !== previousId)) {
    return refused(source, "Each choice needs a unique ID.");
  }
  const current = source.response.choices.find((choice) => choice.id === previousId);
  if (current === undefined) return refused(source, "That choice no longer exists.");
  const choices = source.response.choices.map((choice) =>
    choice.id === previousId ? { ...choice, id: nextId } : choice,
  );
  const correctChoice =
    source.response.correctChoice === previousId ? nextId : source.response.correctChoice;
  return changed({ ...source, response: { ...source.response, choices, correctChoice } });
}

export function setOutcomeFeedback(
  source: FlatQuestionSourceV2,
  feedback: FlatQuestionOutcomeFeedback,
): FlatQuestionSourceV2 {
  return { ...source, feedback };
}

export function setAttemptPolicy(
  source: FlatQuestionSourceV2,
  attemptPolicy: FlatQuestionAttemptPolicy,
): FlatQuestionSourceV2 {
  return { ...source, attemptPolicy };
}

export function setTimingPolicy(
  source: FlatQuestionSourceV2,
  timingPolicy: FlatQuestionTimingPolicy,
): FlatQuestionSourceV2 {
  return { ...source, timingPolicy };
}

export function setTags(
  source: FlatQuestionSourceV2,
  tags: ReadonlyArray<string>,
): FlatQuestionSourceV2 {
  return { ...source, tags: [...tags] };
}

export function setTaxonomy(
  source: FlatQuestionSourceV2,
  taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>,
): FlatQuestionSourceV2 {
  return { ...source, taxonomy: [...taxonomy] };
}

export function setLicense(
  source: FlatQuestionSourceV2,
  license: FlatQuestionLicense,
): FlatQuestionSourceV2 {
  return { ...source, license };
}

export function setLanguage(source: FlatQuestionSourceV2, language: string): FlatQuestionSourceV2 {
  return { ...source, language };
}

/** Uses the canonical codec and turns its structural result into safe field-level author guidance. */
export function validateFlatQuestionSource(source: FlatQuestionSourceV2): FlatQuestionValidation {
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
export function sourcesEqual(left: FlatQuestionSourceV2, right: FlatQuestionSourceV2): boolean {
  return stableSourceText(left) === stableSourceText(right);
}

function stableSourceText(source: FlatQuestionSourceV2): string {
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
  if (field.startsWith("response.choices"))
    return "Check the choices and select one correct answer.";
  if (field.startsWith("title")) return "Add a short question title.";
  if (field.startsWith("prompt")) return "Add the Student-facing question prompt.";
  if (field.startsWith("points")) return "Points must be a nonnegative number.";
  if (field.startsWith("timingPolicy")) return "Check the timing policy values.";
  return "Check the question details before saving.";
}
