// One lazy prompt-only adapter for already-loaded exact Question Revisions.

import type { JSX } from "solid-js";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import { QuestionPromptRenderer } from "./question_renderer";

export interface QuestionNativePreview {
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly nativePrompt: ReadonlyArray<QuestionContentBlock> | null;
}

/** Creates content only when RecordItem mounts its preview under a Solid owner. */
export function questionNativePreview(
  question: QuestionNativePreview,
): (() => JSX.Element) | undefined {
  const blocks = question.nativePrompt;
  if (blocks === null) return undefined;
  const tuple = question.publishedQuestionRevisionTuple;
  return () => (
    <div class="question-renderer question-renderer__prompt">
      <QuestionPromptRenderer
        blocks={blocks}
        publishedQuestionRevisionTuple={tuple}
        // ASVS 1.3.1: canonical renderer sanitizes markup and enforces this exact same-origin route.
        questionImageUrl={(image) =>
          new URL(
            `/api/questions/${encodeURIComponent(tuple.publishedQuestionId)}/revisions/${tuple.revisionNumber}/images/${encodeURIComponent(image.questionImageAssetId)}`,
            globalThis.location.origin,
          )
        }
      />
    </div>
  );
}

/** Exact revision key for UI recognition metadata; never a Question Library head lookup. */
export function questionNativePreviewKey(tuple: PublishedQuestionRevisionTuple): string {
  return `${tuple.publishedQuestionId}:${tuple.revisionNumber}`;
}
