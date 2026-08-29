// run_summary_page.tsx - bounded, server-projected learner run history.

import { useNavigate } from "@solidjs/router";
import { createSignal, For, onMount, Show, type JSX } from "solid-js";

import type { RunSummaryOutcome, RunSummaryResponse } from "../api/contracts";
import { FeedbackPanel } from "../components/feedback_panel";
import { useApiRuntime } from "../api/runtime";
import { useCourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import { runRouteReference } from "../navigation/public_route";
import { studentProgressSummary, studentScoreValue } from "../student_progress";

export function RunSummaryPage(): JSX.Element {
  const runtime = useApiRuntime();
  const navigate = useNavigate();
  const scopedRoute = useCourseThemeRouteData();
  const initialSummary = scopedRoute?.kind === "runSummary" ? scopedRoute.response : undefined;
  const [summary, setSummary] = createSignal<RunSummaryResponse | undefined>(initialSummary);
  const [rows, setRows] = createSignal<ReadonlyArray<RunSummaryOutcome>>(
    initialSummary?.outcomes.items ?? [],
  );
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [assignmentId, setAssignmentId] = createSignal<string>();
  const [practiceError, setPracticeError] = createSignal<string | null>(null);
  const seen = new Set<string>();

  async function loadAssignment(next: RunSummaryResponse): Promise<void> {
    const enrollment = await runtime.client.getEnrollment(next.run.enrollment);
    setAssignmentId(enrollment.enrollment.assignment);
  }

  async function load(cursor?: string): Promise<void> {
    const runId = summary()?.run.id ?? initialSummary?.run.id;
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
      await loadAssignment(next);
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

  onMount(() => {
    if (initialSummary === undefined) {
      void load();
      return;
    }
    void loadAssignment(initialSummary).catch(() => {
      setError("Could not restore assignment actions. Your displayed summary is still available.");
    });
  });
  async function startPractice(): Promise<void> {
    const courseId = summary()?.course.summary.id;
    const assignment = assignmentId();
    if (courseId === undefined || assignment === undefined) return;
    setPracticeError(null);
    try {
      const run = await runtime.client.startRun(courseId, assignment);
      navigate(`/runs/${runRouteReference(run.reference)}`);
    } catch {
      setPracticeError("Could not start a fresh practice run. Your summary is still available.");
    }
  }
  return (
    <section class="page attempt-summary" data-route-surface="runSummary">
      <p class="eyebrow">Completed practice run</p>
      <h1>Run summary</h1>
      <Show
        when={summary()}
        fallback={<p class="loading-state">Loading your recorded responses...</p>}
      >
        {(current) => (
          <>
            <p>Your completed run is recorded.</p>
            <Show when={current().practiceAllowed}>
              <p>You can start a fresh practice run from your assignment.</p>
            </Show>
            <section aria-label="Assignment score">
              <h2>Assignment score</h2>
              <p>{studentProgressSummary(current().summary)}</p>
              <Show when={current().summary.score_state === "available"}>
                <p>This run: {studentScoreValue(current().run.score)}</p>
              </Show>
            </section>
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
                      ? {
                          kind: "awaiting",
                          feedback: null,
                          scoringStatus: outcome.scoringStatus,
                        }
                      : {
                          kind: "released",
                          feedback: outcome.feedback,
                          scoringStatus: outcome.scoringStatus,
                        }
                  }
                  assetUrl={(asset) =>
                    new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                  }
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
