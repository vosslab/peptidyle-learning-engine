import type {
  FlatQuestionAttemptPolicy,
  FlatQuestionLicense,
  FlatQuestionSourceV2,
  FlatQuestionTaxonomyTerm,
  FlatQuestionTimingPolicy,
} from "./flat_question_source";

/** The local learner preview deliberately excludes correctness and all feedback. */
export type FlatQuestionPublicPreview = {
  readonly title: string;
  readonly prompt: string;
  readonly choices: ReadonlyArray<{ readonly id: string; readonly text: string }>;
  readonly points: number;
  readonly attemptPolicy: FlatQuestionAttemptPolicy;
  readonly timingPolicy: FlatQuestionTimingPolicy;
  readonly tags: ReadonlyArray<string>;
  readonly taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>;
  readonly license: FlatQuestionLicense;
  readonly language: string;
};

/** Projects an author source into exactly the information a learner may receive. */
export function flatQuestionPublicPreview(source: FlatQuestionSourceV2): FlatQuestionPublicPreview {
  const choices = source.response.choices.map((choice) => ({ id: choice.id, text: choice.text }));
  return {
    title: source.title,
    prompt: source.prompt,
    choices,
    points: source.points,
    attemptPolicy: source.attemptPolicy,
    timingPolicy: source.timingPolicy,
    tags: source.tags,
    taxonomy: source.taxonomy,
    license: source.license,
    language: source.language,
  };
}

/** Serializes only the answer-free local preview, suitable for boundary tests. */
export function serializeFlatQuestionPublicPreview(source: FlatQuestionSourceV2): string {
  return JSON.stringify(flatQuestionPublicPreview(source));
}
