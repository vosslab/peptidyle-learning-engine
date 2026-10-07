import { A } from "@solidjs/router";
import { createSignal, Show, type JSX } from "solid-js";

import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import { ApiRequestError } from "../../api/http_client/error";
import { buildRoutePath } from "../../ribbon/ribbon_route_path";
import {
  createDraftQuestionPreviewClient,
  type DraftQuestionPreviewClient,
} from "./draft_preview_client";

const MAX_REASON_SCALARS = 2_000;

export type DraftCorrectionPublishControlProps = {
  readonly draftQuestion: DraftQuestionRouteId;
  readonly parent: PublishedQuestionRevisionTuple;
  readonly draftQuestionEditNumber: string;
  readonly isSaved: boolean;
  readonly client?: DraftQuestionPreviewClient;
};

function safeError(error: unknown): string {
  if (error instanceof ApiRequestError) {
    if ([409, 412, 428].includes(error.status))
      return "The Draft changed before publication. Reload it, review your edits, then try again.";
    if (error.status === 422)
      return "The existing revision publisher could not publish this Draft source yet. The saved Draft remains available for editing.";
  }
  return "The correction could not be published. The saved Draft remains available for editing.";
}

/** Publishes a saved correction through the existing /publish-revision operation. */
export function DraftCorrectionPublishControl(
  props: DraftCorrectionPublishControlProps,
): JSX.Element {
  const client = props.client ?? createDraftQuestionPreviewClient();
  const [reason, setReason] = createSignal("");
  const [publishing, setPublishing] = createSignal(false);
  const [error, setError] = createSignal("");
  const [published, setPublished] = createSignal<PublishedQuestionRevisionTuple>();
  const validReason = (): boolean => {
    const value = reason();
    return (
      value.length > 0 &&
      value.trim() === value &&
      Array.from(value).length <= MAX_REASON_SCALARS &&
      !Array.from(value).some((character) => {
        const codePoint = character.codePointAt(0);
        return codePoint !== undefined && (codePoint < 0x20 || codePoint === 0x7f);
      })
    );
  };

  async function publishCorrection(): Promise<void> {
    if (!props.isSaved || publishing() || !validReason()) return;
    setPublishing(true);
    setError("");
    setPublished(undefined);
    try {
      const result = await client.publishRevision(
        props.draftQuestion,
        props.parent,
        reason(),
        props.draftQuestionEditNumber,
      );
      if (
        result.publishedQuestionId !== props.parent.publishedQuestionId ||
        result.revisionNumber !== props.parent.revisionNumber + 1
      ) {
        throw new Error("The revision result did not match this correction Draft.");
      }
      setPublished(result);
    } catch (caught: unknown) {
      setError(safeError(caught));
    } finally {
      setPublishing(false);
    }
  }

  return (
    <section
      class="editor-panel draft-correction-publish"
      aria-labelledby="draft-correction-heading"
    >
      <h2 id="draft-correction-heading">Publish this correction</h2>
      <p>
        This creates the next immutable Revision on the same Published Question through the existing
        revision operation.
      </p>
      <label class="ple-question-json-authoring__field">
        <span>Reason for edit</span>
        <textarea
          value={reason()}
          disabled={publishing() || published() !== undefined}
          maxLength={MAX_REASON_SCALARS}
          onInput={(event) => setReason(event.currentTarget.value)}
        />
      </label>
      <Show when={!props.isSaved}>
        <p role="status">Save all source and support changes before publishing the correction.</p>
      </Show>
      <button
        type="button"
        class="primary-action"
        disabled={!props.isSaved || publishing() || !validReason() || published() !== undefined}
        onClick={() => void publishCorrection()}
      >
        {publishing() ? "Publishing correction..." : "Publish correction"}
      </button>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
      <Show when={published()}>
        {(revision) => (
          <p role="status">
            Correction published as Revision {revision().revisionNumber}.{" "}
            <A
              href={
                buildRoutePath("questionDetail", {
                  questionId: revision().publishedQuestionId,
                }) ?? "/library/browse"
              }
            >
              Open the revised Question
            </A>
          </p>
        )}
      </Show>
    </section>
  );
}
