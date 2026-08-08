// run_summary_page.tsx - bounded, server-projected learner run history.

import { useNavigate, useParams } from "@solidjs/router";
import { createSignal, For, onMount, Show, type JSX } from "solid-js";

import type { RunSummaryOutcome, RunSummaryResponse } from "../api/contracts";
import { FeedbackPanel } from "../components/feedback_panel";
import { useApiRuntime } from "../api/runtime";

export function RunSummaryPage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const navigate = useNavigate();
  const [summary, setSummary] = createSignal<RunSummaryResponse>();
  const [rows, setRows] = createSignal<ReadonlyArray<RunSummaryOutcome>>([]);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [assignmentId, setAssignmentId] = createSignal<string>();
  const [practiceError, setPracticeError] = createSignal<string | null>(null);
  const seen = new Set<string>();

  async function load(cursor?: string): Promise<void> {
    const runId = params["runId"];
    if (runId === undefined || loading() || (cursor !== undefined && seen.has(cursor))) return;
    if (cursor === undefined) seen.clear();
    setLoading(true);
    setError(null);
    try {
      const next = await runtime.client.getRunSummary(runId, cursor, 30);
      if (next.outcomes.nextCursor !== null && seen.has(next.outcomes.nextCursor)) {
        throw new Error("Repeated summary cursor");
      }
      if (cursor !== undefined) seen.add(cursor);
      setSummary(next);
      const enrollment = await runtime.client.getEnrollment(next.run.enrollment);
      setAssignmentId(enrollment.enrollment.assignment);
      setRows((prior) => {
        const base = cursor === undefined ? [] : prior;
        const ids = new Set(base.map((row) => row.attempt));
        return [...base, ...next.outcomes.items.filter((row) => !ids.has(row.attempt))];
      });
    } catch {
      setError("Could not load more responses. Your displayed summary is still available.");
    } finally {
      setLoading(false);
    }
  }

  onMount(() => void load());
  async function startPractice(): Promise<void> {
    const assignment = assignmentId();
    if (assignment === undefined) return;
    setPracticeError(null);
    try {
      const run = await runtime.client.startRun(assignment);
      navigate(`/runs/${run.id}`);
    } catch {
      setPracticeError("Could not start a fresh practice run. Your summary is still available.");
    }
  }
  return (
    <section class="page" data-route-surface="runSummary">
      <p class="eyebrow">Completed practice run</p>
      <h1>Run summary</h1>
      <Show
        when={summary()}
        fallback={<p class="loading-state">Loading your recorded responses...</p>}
      >
        {(current) => (
          <>
            <p>
              {current().practiceAllowed
                ? "You can start a fresh practice run from your assignment."
                : "This run is recorded."}
            </p>
            <Show when={current().practiceAllowed && assignmentId() !== undefined}>
              <button class="primary-action" type="button" onClick={() => void startPractice()}>
                Start fresh practice
              </button>
            </Show>
            <Show when={practiceError()}>
              {(message) => (
                <>
                  <p class="inline-error">{message()}</p>
                  <button class="quiet-action" type="button" onClick={() => void startPractice()}>
                    Retry starting practice
                  </button>
                </>
              )}
            </Show>
            <For each={rows()}>
              {(outcome) => (
                <FeedbackPanel
                  disclosure={
                    outcome.feedback === null
                      ? { kind: "awaiting", feedback: null }
                      : { kind: "released", feedback: outcome.feedback }
                  }
                  assetUrl={(asset) =>
                    new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                  }
                  onAdvance={() => {}}
                  advanceLabel="Stay on summary"
                />
              )}
            </For>
            <Show when={current().outcomes.nextCursor !== null}>
              <button
                class="quiet-action"
                type="button"
                disabled={loading()}
                onClick={() => void load(current().outcomes.nextCursor ?? undefined)}
              >
                Load more responses
              </button>
            </Show>
          </>
        )}
      </Show>
      <Show when={error()}>
        {(message) => (
          <>
            <p class="inline-error">{message()}</p>
            <button
              class="quiet-action"
              type="button"
              onClick={() => void load(summary()?.outcomes.nextCursor ?? undefined)}
            >
              Retry
            </button>
          </>
        )}
      </Show>
    </section>
  );
}
