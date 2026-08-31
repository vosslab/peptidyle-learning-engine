import type {
  FlatQuestionAttemptLimit,
  FlatQuestionChoice,
  FlatQuestionLicense,
  FlatQuestionSourceV2,
  FlatQuestionTaxonomyTerm,
  FlatQuestionAttemptTimeLimit,
} from "./flat_question_source";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";

/** The local student preview deliberately excludes correctness and all feedback. */
export type FlatQuestionPublicPreview = {
  readonly title: string;
  readonly prompt: string;
  readonly response: QuestionResponseFormat;
  readonly points: number;
  readonly questionAttemptLimit: FlatQuestionAttemptLimit;
  readonly questionAttemptTimeLimit: FlatQuestionAttemptTimeLimit;
  readonly tags: ReadonlyArray<string>;
  readonly taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>;
  readonly license: FlatQuestionLicense;
  readonly language: string;
};

/** Projects an author source into exactly the information a student may receive. */
export function flatQuestionPublicPreview(source: FlatQuestionSourceV2): FlatQuestionPublicPreview {
  const response = flatQuestionResponseFormat(source);
  return {
    title: source.title,
    prompt: source.prompt,
    response,
    points: source.points,
    questionAttemptLimit: source.questionAttemptLimit,
    questionAttemptTimeLimit: source.questionAttemptTimeLimit,
    tags: source.tags,
    taxonomy: source.taxonomy,
    license: source.license,
    language: source.language,
  };
}

/** Builds the existing key-free runtime definition; source answer material never crosses this seam. */
export function flatQuestionResponseFormat(source: FlatQuestionSourceV2): QuestionResponseFormat {
  const response = source.response;
  switch (response.kind) {
    case "singleChoice":
      return choiceDefinition(response.choices, { kind: "exactlyOne" });
    case "multipleAnswer":
      return choiceDefinition(response.choices, { kind: "atLeastOne" });
    case "fillIn":
      return {
        kind: "shortText",
        matchMode: response.matchMode,
        maxLength: response.maxLength,
      };
    case "multiFillIn":
      return {
        kind: "multiBlank",
        blanks: response.blanks.map((blank) => ({
          id: blank.id,
          label: [{ kind: "text", markdown: blank.label }],
          matchMode: blank.matchMode,
          maxLength: blank.maxLength,
        })),
      };
    case "numeric":
      return { kind: "numeric", tolerance: response.tolerance, unit: response.unit };
    case "matching":
      return {
        kind: "matching",
        prompts: response.prompts.map((item) => ({
          id: item.id,
          body: [{ kind: "text", markdown: item.text }],
        })),
        choices: response.choices.map((item) => ({
          id: item.id,
          body: [{ kind: "text", markdown: item.text }],
        })),
      };
    case "ordering":
      return {
        kind: "ordering",
        items: response.items.map((item) => ({
          id: item.id,
          body: [{ kind: "text", markdown: item.text }],
        })),
      };
    case "hotspot":
      return {
        kind: "hotspot",
        surface: { asset: response.surface.asset, checksum: response.surface.checksum },
        description: response.surface.description,
        regions: response.regions.map((region) => ({
          id: region.id,
          label: [{ kind: "text", markdown: region.label }],
          x: region.x,
          y: region.y,
          width: region.width,
          height: region.height,
        })),
        // The private correct-region set must not determine public response shape.
        // Students may choose one or more public candidate regions; server-only
        // grading decides whether their complete selection is correct.
        selection: { kind: "atLeastOne" },
      };
  }
}

function choiceDefinition(
  choices: ReadonlyArray<FlatQuestionChoice>,
  selection: { readonly kind: "exactlyOne" | "atLeastOne" },
): QuestionResponseFormat {
  return {
    kind: "multipleChoice",
    choices: choices.map((choice) => ({
      id: choice.id,
      body: [{ kind: "text", markdown: choice.text }],
    })),
    selection,
  };
}

/** Serializes only the answer-free local preview, suitable for boundary tests. */
export function serializeFlatQuestionPublicPreview(source: FlatQuestionSourceV2): string {
  return JSON.stringify(flatQuestionPublicPreview(source));
}
