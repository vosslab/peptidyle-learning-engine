// assignment_attempt_summary_page.tsx - bounded, server-projected Assignment Attempt history.

import { useNavigate } from "@solidjs/router";
import { createSignal, For, onMount, Show, type JSX } from "solid-js";

import type {
  AssignmentAttemptSummaryOutcome,
  AssignmentAttemptSummaryResponse,
} from "../api/contracts";
import { StudentFeedbackPanel } from "../components/student_feedback_panel";
import { useApplicationApi } from "../api/application_api";
import { useCourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import { assignmentAttemptRouteReference } from "../navigation/public_route";
import { studentProgressSummary, studentScoreValue } from "../student_progress";

export function AssignmentAttemptSummaryPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const scopedRoute = useCourseThemeRouteData();
  const initialSummary =
    scopedRoute?.kind === "assignmentAttemptSummary" ? scopedRoute.response : undefined;
  const [summary, setSummary] = createSignal<AssignmentAttemptSummaryResponse | undefined>(
    initialSummary,
  );
  const [rows, setRows] = createSignal<ReadonlyArray<AssignmentAttemptSummaryOutcome>>(
    initialSummary?.outcomes.items ?? [],
  );
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const seen = new Set<string>();

  async function load(cursor?: string): Promise<void> {
    const assignmentAttemptId =
      summary()?.assignmentAttempt.id ?? initialSummary?.assignmentAttempt.id;
    if (
      assignmentAttemptId === undefined ||
      loading() ||
      (cursor !== undefined && seen.has(cursor))
    ) {
      return;
    }
    if (cursor === undefined) seen.clear();
    setLoading(true);
    setError(null);
    try {
      const next = await applicationApi.client.getAssignmentAttemptSummary(
        assignmentAttemptId,
        cursor,
        30,
      );
      if (next.outcomes.nextCursor !== null && seen.has(next.outcomes.nextCursor)) {
        throw new Error("Repeated summary cursor");
      }
      if (cursor !== undefined) seen.add(cursor);
      setSummary(next);
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
    return undefined;
  });
  async function startAnotherAssignmentAttempt(): Promise<void> {
    const courseId = summary()?.course.summary.id;
    const assignment = summary()?.assignmentAttempt.assignment;
    if (courseId === undefined || assignment === undefined) return;
    try {
      const assignmentAttempt = await applicationApi.client.startAssignmentAttempt(
        courseId,
        assignment,
      );
      navigate(
        `/assignment-attempts/${assignmentAttemptRouteReference(assignmentAttempt.reference)}`,
      );
    } catch {
      setError("Could not start another Assignment Attempt. Your summary is still available.");
    }
  }
  return (
    <section class="page attempt-summary" data-route-surface="assignmentAttemptSummary">
      <p class="eyebrow">Completed Assignment Attempt</p>
      <h1>Assignment Attempt summary</h1>
      <Show
        when={summary()}
        fallback={<p class="loading-state">Loading your recorded responses...</p>}
      >
        {(current) => (
          <>
            <p>Your completed Assignment Attempt is recorded.</p>
            <section aria-label="Assignment score">
              <h2>Assignment score</h2>
              <p>{studentProgressSummary(current().summary)}</p>
              <Show when={current().summary.student_assignment_grade.score_state === "available"}>
                <p>
                  This Assignment Attempt: {studentScoreValue(current().assignmentAttempt.score)}
                </p>
              </Show>
            </section>
            <button
              class="primary-action"
              type="button"
              onClick={() => void startAnotherAssignmentAttempt()}
            >
              Start another Assignment Attempt
            </button>
            <For each={rows()}>
              {(outcome) => (
                <StudentFeedbackPanel
                  disclosure={
                    outcome.feedback === null
                      ? {
                          kind: "awaiting",
                          feedback: null,
                          assignmentScoringState: outcome.assignmentScoringState,
                        }
                      : {
                          kind: "released",
                          feedback: outcome.feedback,
                          assignmentScoringState: outcome.assignmentScoringState,
                        }
                  }
                  assetUrl={(asset) =>
                    new URL(applicationApi.client.assetUrl(asset.questionAsset), window.location.origin)
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
