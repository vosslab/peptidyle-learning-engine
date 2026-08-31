// editor_page_typecheck.ts - compile-time boundary proof for unversioned workspace preview.

import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { QuestionPresentation as IssuedQuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { QuestionPresentation } from "../components/question_renderer";
import type { EditorPreview } from "./editor_page_model";

/** The renderer receives only identity-free content from a workspace preview. */
export function assertPreviewPresentationBoundary(preview: EditorPreview): QuestionPresentation {
  const presentation: QuestionPresentation = preview;

  // @ts-expect-error A workspace preview cannot enter a published-envelope path.
  const invalidEnvelope: IssuedQuestionPresentation = preview;
  // @ts-expect-error A workspace preview cannot enter an assignment's published definition path.
  const invalidDefinition: QuestionDefinition = preview;
  void invalidEnvelope;
  void invalidDefinition;
  return presentation;
}
