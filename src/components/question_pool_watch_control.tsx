// Self-only Watch control for one exact Published Question Pool.

import { Show, createResource, createSignal, type JSX } from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import { useApplicationApi } from "../api/application_api";
import "./question_pool_watch_control.css";

export interface QuestionPoolWatchControlProps {
  readonly poolId: QuestionPoolId;
}

/** Shows only the current active Instructor's Watch state for this Pool. */
export function QuestionPoolWatchControl(props: QuestionPoolWatchControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [watch, { mutate, refetch }] = createResource(
    () => props.poolId,
    (poolId) => applicationApi.client.getQuestionPoolWatch(poolId),
  );
  const [saving, setSaving] = createSignal(false);
  const [saveError, setSaveError] = createSignal(false);

  async function toggleWatch(): Promise<void> {
    const current = watch();
    if (current === undefined || saving()) return;
    setSaving(true);
    setSaveError(false);
    try {
      mutate(await applicationApi.client.setQuestionPoolWatch(props.poolId, !current.watching));
    } catch {
      setSaveError(true);
    } finally {
      setSaving(false);
    }
  }

  return (
    <section
      class="library-pool-watch-control"
      aria-label="Question Pool Watch"
      aria-busy={saving() || watch.loading}
    >
      <Show when={watch.loading}>
        <p role="status">Loading private Pool Watch state...</p>
      </Show>
      <Show when={watch.error !== undefined}>
        <p role="alert">Your Pool Watch state could not load. Try again.</p>
        <button type="button" onClick={() => void refetch()}>
          Retry Pool Watch
        </button>
      </Show>
      <Show when={watch.error === undefined ? watch() : undefined}>
        {(projection) => (
          <button
            type="button"
            classList={{ selected: projection().watching }}
            aria-pressed={projection().watching}
            aria-disabled={saving()}
            onClick={() => void toggleWatch()}
          >
            {projection().watching ? "Unwatch Pool" : "Watch Pool"}
          </button>
        )}
      </Show>
      <Show when={saveError()}>
        <p role="alert">Your Pool Watch setting could not be saved. Try again.</p>
      </Show>
    </section>
  );
}
