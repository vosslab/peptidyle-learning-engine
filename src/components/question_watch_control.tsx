// Self-only Watch control for a single Published Question detail surface.

import { createResource, createSignal, type JSX, Show } from "solid-js";

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import { useApplicationApi } from "../api/application_api";

export interface QuestionWatchControlProps {
  readonly questionId: PublishedQuestionId;
}

/** Shows only the current active Instructor's Watch state for this Question. */
export function QuestionWatchControl(props: QuestionWatchControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [watch, { mutate }] = createResource(
    () => props.questionId,
    (questionId) => applicationApi.client.getQuestionWatch(questionId),
  );
  const [saving, setSaving] = createSignal(false);

  async function toggleWatch(): Promise<void> {
    const current = watch();
    if (current === undefined || saving()) return;
    setSaving(true);
    try {
      mutate(await applicationApi.client.setQuestionWatch(props.questionId, !current.watching));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Show when={watch()}>
      {(projection) => (
        <section class="question-watch-control" aria-label="Question Watch">
          <button
            type="button"
            class="question-preference-control"
            classList={{ selected: projection().watching }}
            aria-pressed={projection().watching}
            disabled={saving()}
            onClick={() => void toggleWatch()}
          >
            {projection().watching ? "Unwatch question" : "Watch question"}
          </button>
        </section>
      )}
    </Show>
  );
}
