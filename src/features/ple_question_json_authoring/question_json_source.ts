import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionCitation } from "../../../generated/api/QuestionCitation";

/**
 * The answer-bearing PLE ple-question-json v2 authoring contract. The server
 * derives its separate PLE Question JSON Public Content Checksum when it
 * compiles the private Answer Key and Question Feedback binding.
 *
 * This type stays inside the authoring feature.  Student-facing code uses the
 * answer-free PLE Question JSON Public Preview from question_json_public_preview.ts instead.
 */

export const PLE_QUESTION_JSON_FORMAT = "pleQuestionJson";
export const PLE_QUESTION_JSON_SCHEMA_VERSION = 2;
export const PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND = "singleChoice" as const;
export const PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND = "matching" as const;
export const PLE_QUESTION_JSON_MULTIPLE_ANSWER_RESPONSE_KIND = "multipleAnswer" as const;
export const PLE_QUESTION_JSON_FILL_IN_RESPONSE_KIND = "fillIn" as const;
export const PLE_QUESTION_JSON_MULTI_FILL_IN_RESPONSE_KIND = "multiFillIn" as const;
export const PLE_QUESTION_JSON_NUMERIC_RESPONSE_KIND = "numeric" as const;
export const PLE_QUESTION_JSON_ORDERING_RESPONSE_KIND = "ordering" as const;
export const PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND = "hotspot" as const;
/** Compatibility name for the initial single-choice default only. */
export const PLE_QUESTION_JSON_MEDIA_TYPE = "application/vnd.peptidyle.question+json";

export type PleQuestionJsonAttemptLimit = {
  readonly maxAttempts: number | null;
};

export type PleQuestionJsonAttemptTimeLimit =
  | { readonly kind: "unlimited" }
  | { readonly kind: "limited"; readonly seconds: number; readonly graceSeconds: number };

export type PleQuestionJsonChoice = {
  readonly id: string;
  readonly text: string;
  /** Null is canonical when a choice has no private Choice Feedback. */
  readonly feedback: string | null;
};

export type PleQuestionJsonOutcomeFeedback = {
  /** Null is canonical when an outcome has no additional feedback. */
  readonly correct: string | null;
  readonly incorrect: string | null;
};

declare const PLE_QUESTION_JSON_MATCHING_PROMPT_ROLE: unique symbol;
declare const PLE_QUESTION_JSON_MATCHING_CHOICE_ROLE: unique symbol;
declare const PLE_QUESTION_JSON_ORDERING_ITEM_ROLE: unique symbol;

type PleQuestionJsonResponseMember = {
  readonly id: string;
  readonly text: string;
};

/** A stable semantic identifier keeps a matching prompt meaningful when an author reorders it. */
export type PleQuestionJsonMatchingPrompt = PleQuestionJsonResponseMember & {
  readonly [PLE_QUESTION_JSON_MATCHING_PROMPT_ROLE]: "matchingPrompt";
};

export function createPleQuestionJsonMatchingPrompt(
  id: string,
  text: string,
): PleQuestionJsonMatchingPrompt {
  return { id, text } as PleQuestionJsonMatchingPrompt;
}

/** A stable semantic identifier keeps a matching choice meaningful when an author reorders it. */
export type PleQuestionJsonMatchingChoice = PleQuestionJsonResponseMember & {
  readonly [PLE_QUESTION_JSON_MATCHING_CHOICE_ROLE]: "matchingChoice";
};

export function createPleQuestionJsonMatchingChoice(
  id: string,
  text: string,
): PleQuestionJsonMatchingChoice {
  return { id, text } as PleQuestionJsonMatchingChoice;
}

/** A stable semantic identifier keeps a correct ordering meaningful when an author reorders it. */
export type PleQuestionJsonOrderingItem = PleQuestionJsonResponseMember & {
  readonly [PLE_QUESTION_JSON_ORDERING_ITEM_ROLE]: "orderingItem";
};

export function createPleQuestionJsonOrderingItem(
  id: string,
  text: string,
): PleQuestionJsonOrderingItem {
  return { id, text } as PleQuestionJsonOrderingItem;
}

/** This private pair map is deliberately absent from the PLE Question JSON Public Preview. */
export type PleQuestionJsonMatch = {
  readonly prompt: string;
  readonly choice: string;
};

export type PleQuestionJsonTextResponseMatchRule = "exact" | "caseInsensitive" | "normalized";

export type PleQuestionJsonBlank = {
  readonly id: string;
  readonly label: string;
  readonly answers: ReadonlyArray<string>;
  readonly matchMode: PleQuestionJsonTextResponseMatchRule;
  readonly maxLength: number;
};

export type PleQuestionJsonNumericResponseTolerance =
  | { readonly kind: "exact" }
  | { readonly kind: "absolute"; readonly epsilon: number }
  | { readonly kind: "relative"; readonly fraction: number }
  | { readonly kind: "significantFigures"; readonly digits: number };

/** Immutable object reference and accessible description for a hotspot surface. */
export type PleQuestionJsonHotspotSurface = {
  readonly asset: string;
  readonly checksum: string;
  readonly description: string;
};

/** Normalized coordinates keep region identity independent of display layout. */
export type PleQuestionJsonHotspotRegion = {
  readonly id: string;
  readonly label: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
};

/** Exact JSON source shape accepted by the private authoring endpoint. */
export type PleQuestionJsonSingleChoiceResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND;
  readonly choices: ReadonlyArray<PleQuestionJsonChoice>;
  readonly correctChoice: string;
};

export type PleQuestionJsonMatchingResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND;
  readonly prompts: ReadonlyArray<PleQuestionJsonMatchingPrompt>;
  readonly choices: ReadonlyArray<PleQuestionJsonMatchingChoice>;
  readonly matches: ReadonlyArray<PleQuestionJsonMatch>;
};

export type PleQuestionJsonMultipleAnswerResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_MULTIPLE_ANSWER_RESPONSE_KIND;
  readonly choices: ReadonlyArray<PleQuestionJsonChoice>;
  readonly correctChoices: ReadonlyArray<string>;
};

export type PleQuestionJsonFillInResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_FILL_IN_RESPONSE_KIND;
  readonly answers: ReadonlyArray<string>;
  readonly matchMode: PleQuestionJsonTextResponseMatchRule;
  readonly maxLength: number;
};

export type PleQuestionJsonMultiFillInResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_MULTI_FILL_IN_RESPONSE_KIND;
  readonly blanks: ReadonlyArray<PleQuestionJsonBlank>;
};

export type PleQuestionJsonNumericResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_NUMERIC_RESPONSE_KIND;
  readonly answer: number;
  readonly tolerance: PleQuestionJsonNumericResponseTolerance;
  readonly unit: string | null;
};

export type PleQuestionJsonOrderingResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_ORDERING_RESPONSE_KIND;
  readonly items: ReadonlyArray<PleQuestionJsonOrderingItem>;
  readonly correctOrder: ReadonlyArray<string>;
};

export type PleQuestionJsonHotspotResponse = {
  readonly kind: typeof PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND;
  readonly surface: PleQuestionJsonHotspotSurface;
  readonly regions: ReadonlyArray<PleQuestionJsonHotspotRegion>;
  readonly correctRegions: ReadonlyArray<string>;
};

export type PleQuestionJsonResponse =
  | PleQuestionJsonSingleChoiceResponse
  | PleQuestionJsonMultipleAnswerResponse
  | PleQuestionJsonFillInResponse
  | PleQuestionJsonMultiFillInResponse
  | PleQuestionJsonNumericResponse
  | PleQuestionJsonMatchingResponse
  | PleQuestionJsonOrderingResponse
  | PleQuestionJsonHotspotResponse;

export type PleQuestionJsonDocument = {
  readonly format: typeof PLE_QUESTION_JSON_FORMAT;
  readonly version: typeof PLE_QUESTION_JSON_SCHEMA_VERSION;
  readonly title: string;
  /** Instructor-facing discovery summary, excluded from the student preview. */
  readonly questionDescription: string;
  readonly prompt: string;
  readonly response: PleQuestionJsonResponse;
  /** Learner-requested instructional support before a response; separate from outcome feedback. */
  readonly questionHint: string | null;
  readonly feedback: PleQuestionJsonOutcomeFeedback;
  readonly points: number;
  readonly questionAttemptLimit: PleQuestionJsonAttemptLimit;
  readonly questionAttemptTimeLimit: PleQuestionJsonAttemptTimeLimit;
  readonly tags: ReadonlyArray<string>;
  /** Unset drafts remain editable; publication requires an exact Question License. */
  readonly questionLicense: QuestionLicense | null;
  readonly questionCitation: QuestionCitation | null;
  readonly language: string;
};
