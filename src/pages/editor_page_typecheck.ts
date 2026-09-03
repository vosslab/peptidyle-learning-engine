// editor_page_typecheck.ts - compile-time boundary proof for unversioned workspace preview.

import type { QuestionRevision } from "../../generated/api/QuestionRevision";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { QuestionVariationPresentation } from "../components/question_renderer";
import type { EditorPreview } from "./editor_page_model";

/** The renderer receives only identity-free content from a workspace preview. */
export function assertPreviewPresentationBoundary(
  preview: EditorPreview,
): QuestionVariationPresentation {
  const presentation: QuestionVariationPresentation = preview;

  // @ts-expect-error A workspace preview cannot enter an issued Question Presentation path.
  const invalidIssuedPresentation: QuestionPresentation = preview;
  // @ts-expect-error A workspace preview cannot enter an assignment's published Question Revision path.
  const invalidVersion: QuestionRevision = preview;
  void invalidIssuedPresentation;
  void invalidVersion;
  return presentation;
}
