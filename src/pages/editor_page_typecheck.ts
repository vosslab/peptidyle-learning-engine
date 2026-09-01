// editor_page_typecheck.ts - compile-time boundary proof for unversioned workspace preview.

import type { QuestionRevision } from "../../generated/api/QuestionRevision";
import type { QuestionVariationPresentation as IssuedQuestionPresentation } from "../../generated/api/QuestionVariationPresentation";
import type { QuestionVariationPresentation } from "../components/question_renderer";
import type { EditorPreview } from "./editor_page_model";

/** The renderer receives only identity-free content from a workspace preview. */
export function assertPreviewPresentationBoundary(
  preview: EditorPreview,
): QuestionVariationPresentation {
  const presentation: QuestionVariationPresentation = preview;

  // @ts-expect-error A workspace preview cannot enter a published-envelope path.
  const invalidEnvelope: IssuedQuestionPresentation = preview;
  // @ts-expect-error A workspace preview cannot enter an assignment's published definition path.
  const invalidVersion: QuestionRevision = preview;
  void invalidEnvelope;
  void invalidVersion;
  return presentation;
}
