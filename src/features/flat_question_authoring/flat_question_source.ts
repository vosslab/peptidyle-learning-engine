/**
 * The answer-bearing PLE flat-question v1 authoring contract.
 *
 * This type stays inside the authoring feature.  Learner-facing code uses the
 * answer-free projection from flat_question_public_preview.ts instead.
 */

export const FLAT_QUESTION_FORMAT = "pleFlatQuestion";
export const FLAT_QUESTION_VERSION = 1;
export const FLAT_QUESTION_KIND = "singleChoice";
export const FLAT_QUESTION_MEDIA_TYPE = "application/vnd.peptidyle.flat-question+json";
export const FLAT_QUESTION_FAMILY = "flat_single_choice_v1";

export type FlatQuestionFeedbackDisclosure =
  "immediateFull" | "immediateCorrectness" | "deferred" | "onRelease";

export type FlatQuestionAttemptPolicy = {
  readonly maxAttempts: number | null;
  readonly feedback: FlatQuestionFeedbackDisclosure;
};

export type FlatQuestionTimingPolicy =
  | { readonly kind: "untimed" }
  | { readonly kind: "perQuestion"; readonly seconds: number; readonly graceSeconds: number }
  | { readonly kind: "perAttempt"; readonly seconds: number; readonly graceSeconds: number };

export type FlatQuestionTaxonomyTerm = {
  readonly scheme: string;
  readonly code: string;
  readonly label: string;
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

/** Exact JSON source shape accepted by the private authoring endpoint. */
export type FlatQuestionSourceV1 = {
  readonly format: typeof FLAT_QUESTION_FORMAT;
  readonly version: typeof FLAT_QUESTION_VERSION;
  readonly kind: typeof FLAT_QUESTION_KIND;
  readonly title: string;
  readonly prompt: string;
  readonly choices: ReadonlyArray<FlatQuestionChoice>;
  readonly correctChoice: string;
  readonly feedback: FlatQuestionOutcomeFeedback;
  readonly points: number;
  readonly attemptPolicy: FlatQuestionAttemptPolicy;
  readonly timingPolicy: FlatQuestionTimingPolicy;
  readonly tags: ReadonlyArray<string>;
  readonly taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>;
  readonly license: FlatQuestionLicense;
  readonly language: string;
};
