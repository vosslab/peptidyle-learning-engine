/**
 * The answer-bearing PLE flat-question v2 authoring contract.
 *
 * This type stays inside the authoring feature.  Student-facing code uses the
 * answer-free projection from flat_question_public_preview.ts instead.
 */

export const FLAT_QUESTION_FORMAT = "pleFlatQuestion";
export const FLAT_QUESTION_VERSION = 2;
export const FLAT_QUESTION_SINGLE_CHOICE_RESPONSE_KIND = "singleChoice" as const;
export const FLAT_QUESTION_MATCHING_RESPONSE_KIND = "matching" as const;
export const FLAT_QUESTION_MULTIPLE_ANSWER_RESPONSE_KIND = "multipleAnswer" as const;
export const FLAT_QUESTION_FILL_IN_RESPONSE_KIND = "fillIn" as const;
export const FLAT_QUESTION_MULTI_FILL_IN_RESPONSE_KIND = "multiFillIn" as const;
export const FLAT_QUESTION_NUMERIC_RESPONSE_KIND = "numeric" as const;
export const FLAT_QUESTION_ORDERING_RESPONSE_KIND = "ordering" as const;
export const FLAT_QUESTION_HOTSPOT_RESPONSE_KIND = "hotspot" as const;
/** Compatibility name for the initial single-choice default only. */
export const FLAT_QUESTION_MEDIA_TYPE = "application/vnd.peptidyle.flat-question+json";

export type FlatQuestionAttemptLimit = {
  readonly maxAttempts: number | null;
};

export type FlatQuestionAttemptTimeLimit =
  | { readonly kind: "unlimited" }
  | { readonly kind: "limited"; readonly seconds: number; readonly graceSeconds: number };

export type FlatQuestionClassification = {
  readonly system: string;
  readonly code: string;
  readonly name: string;
};

export type FlatQuestionLicense =
  | { readonly kind: "allRightsReserved" }
  | { readonly kind: "ccBy" }
  | { readonly kind: "ccBySa" }
  | { readonly kind: "ccByNc" }
  | { readonly kind: "cc0" }
  | { readonly kind: "other"; readonly spdx: string };

export type FlatQuestionChoice = {
  readonly id: string;
  readonly text: string;
  /** Null is canonical when a choice has no private teaching feedback. */
  readonly feedback: string | null;
};

export type FlatQuestionOutcomeFeedback = {
  /** Null is canonical when an outcome has no additional feedback. */
  readonly correct: string | null;
  readonly incorrect: string | null;
};

/** A stable semantic identifier keeps a pairing meaningful when an author reorders either side. */
export type FlatQuestionItem = {
  readonly id: string;
  readonly text: string;
};

/** This private pair map is deliberately absent from the student preview projection. */
export type FlatQuestionMatch = {
  readonly prompt: string;
  readonly choice: string;
};

export type FlatQuestionTextResponseMatchRule = "exact" | "caseInsensitive" | "normalized";

export type FlatQuestionBlank = {
  readonly id: string;
  readonly label: string;
  readonly answers: ReadonlyArray<string>;
  readonly matchMode: FlatQuestionTextResponseMatchRule;
  readonly maxLength: number;
};

export type FlatQuestionNumericResponseTolerance =
  | { readonly kind: "exact" }
  | { readonly kind: "absolute"; readonly epsilon: number }
  | { readonly kind: "relative"; readonly fraction: number }
  | { readonly kind: "significantFigures"; readonly digits: number };

/** Immutable object reference and accessible description for a hotspot surface. */
export type FlatQuestionHotspotSurface = {
  readonly asset: string;
  readonly checksum: string;
  readonly description: string;
};

/** Normalized coordinates keep region identity independent of display layout. */
export type FlatQuestionHotspotRegion = {
  readonly id: string;
  readonly label: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
};

/** Exact JSON source shape accepted by the private authoring endpoint. */
export type FlatQuestionSingleChoiceResponse = {
  readonly kind: typeof FLAT_QUESTION_SINGLE_CHOICE_RESPONSE_KIND;
  readonly choices: ReadonlyArray<FlatQuestionChoice>;
  readonly correctChoice: string;
};

export type FlatQuestionMatchingResponse = {
  readonly kind: typeof FLAT_QUESTION_MATCHING_RESPONSE_KIND;
  readonly prompts: ReadonlyArray<FlatQuestionItem>;
  readonly choices: ReadonlyArray<FlatQuestionItem>;
  readonly matches: ReadonlyArray<FlatQuestionMatch>;
};

export type FlatQuestionMultipleAnswerResponse = {
  readonly kind: typeof FLAT_QUESTION_MULTIPLE_ANSWER_RESPONSE_KIND;
  readonly choices: ReadonlyArray<FlatQuestionChoice>;
  readonly correctChoices: ReadonlyArray<string>;
};

export type FlatQuestionFillInResponse = {
  readonly kind: typeof FLAT_QUESTION_FILL_IN_RESPONSE_KIND;
  readonly answers: ReadonlyArray<string>;
  readonly matchMode: FlatQuestionTextResponseMatchRule;
  readonly maxLength: number;
};

export type FlatQuestionMultiFillInResponse = {
  readonly kind: typeof FLAT_QUESTION_MULTI_FILL_IN_RESPONSE_KIND;
  readonly blanks: ReadonlyArray<FlatQuestionBlank>;
};

export type FlatQuestionNumericResponse = {
  readonly kind: typeof FLAT_QUESTION_NUMERIC_RESPONSE_KIND;
  readonly answer: number;
  readonly tolerance: FlatQuestionNumericResponseTolerance;
  readonly unit: string | null;
};

export type FlatQuestionOrderingResponse = {
  readonly kind: typeof FLAT_QUESTION_ORDERING_RESPONSE_KIND;
  readonly items: ReadonlyArray<FlatQuestionItem>;
  readonly correctOrder: ReadonlyArray<string>;
};

export type FlatQuestionHotspotResponse = {
  readonly kind: typeof FLAT_QUESTION_HOTSPOT_RESPONSE_KIND;
  readonly surface: FlatQuestionHotspotSurface;
  readonly regions: ReadonlyArray<FlatQuestionHotspotRegion>;
  readonly correctRegions: ReadonlyArray<string>;
};

export type FlatQuestionResponse =
  | FlatQuestionSingleChoiceResponse
  | FlatQuestionMultipleAnswerResponse
  | FlatQuestionFillInResponse
  | FlatQuestionMultiFillInResponse
  | FlatQuestionNumericResponse
  | FlatQuestionMatchingResponse
  | FlatQuestionOrderingResponse
  | FlatQuestionHotspotResponse;

export type FlatQuestionSourceV2 = {
  readonly format: typeof FLAT_QUESTION_FORMAT;
  readonly version: typeof FLAT_QUESTION_VERSION;
  readonly title: string;
  readonly prompt: string;
  readonly response: FlatQuestionResponse;
  readonly feedback: FlatQuestionOutcomeFeedback;
  readonly points: number;
  readonly questionAttemptLimit: FlatQuestionAttemptLimit;
  readonly questionAttemptTimeLimit: FlatQuestionAttemptTimeLimit;
  readonly tags: ReadonlyArray<string>;
  readonly classifications: ReadonlyArray<FlatQuestionClassification>;
  readonly license: FlatQuestionLicense;
  readonly language: string;
};
