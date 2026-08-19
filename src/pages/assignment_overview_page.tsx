// assignment_overview_page.tsx - assignment entry and run-resume transition.

import { A, createAsync, useNavigate, useParams } from "@solidjs/router";
import { createSignal, Show, Suspense, type JSX } from "solid-js";

import type { LearnerClassStatistics } from "../../generated/api/LearnerClassStatistics";
import { useApiRuntime } from "../api/runtime";
import { resolveAssignmentRoute } from "../navigation/resolved_route";
import { runRouteReference } from "../navigation/public_route";
import { courseRouteReference, assignmentRouteReference } from "../navigation/public_route";
import { useSessionBootstrap } from "../auth/session_context";
import { learnerProgressSummary, learnerScoreValue } from "../learner_progress";

function formatActivity(timestamp: number | null): string {
  if (timestamp === null) {
    return "No activity yet";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}

function classStatisticsSummary(statistics: LearnerClassStatistics): string {
  if (statistics.state === "insufficientEvidence") {
    return "Not enough evidence to show class statistics.";
  }
  return `Class average: ${learnerScoreValue(
    statistics.assignmentAverageScore,
  )}. Based on ${statistics.completedLearnerCohortSize} completed learners.`;
}

export function AssignmentOverviewPage(): JSX.Element {
  const runtime = useApiRuntime();
  const navigate = useNavigate();
  const params = useParams();
  const [starting, setStarting] = createSignal(false);
  const [startError, setStartError] = createSignal<string>();
  const session = useSessionBootstrap().state();
  const assignment = createAsync(() => {
    return resolveAssignmentRoute(runtime.client, params["assignmentRef"]).then((identity) =>
      runtime.queries.assignment(identity.assignmentId),
    );
  });
  const summary = createAsync(() => {
    const assignmentId = assignment()?.id;
    if (assignmentId === undefined) return Promise.resolve(undefined);
    return runtime.queries.assignmentSummary(assignmentId).catch(() => undefined);
  });
  const course = createAsync(() => {
    return resolveAssignmentRoute(runtime.client, params["assignmentRef"]).then((identity) =>
      runtime.queries.courseScope(identity.courseId),
    );
  });
  const mayEdit = (): boolean =>
    session.kind === "authenticated" &&
    session.session.user.roles.some((role) => role === "instructor" || role === "sysadmin") &&
    course()?.summary.role === "instructor";

  async function startOrResume(): Promise<void> {
    const assignmentId = assignment()?.id;
    if (assignmentId === undefined || starting()) {
      return;
    }
    setStarting(true);
    setStartError(undefined);
    try {
      const run = await runtime.client.startRun(assignmentId);
      navigate(`/runs/${runRouteReference(run.reference)}`);
    } catch (error: unknown) {
      setStartError(
        error instanceof Error
          ? `Could not open the practice run: ${error.message}. Try again.`
          : "Could not open the practice run. Try again.",
      );
      setStarting(false);
    }
  }

  return (
    <section class="page" data-route-surface="assignmentOverview">
      <Suspense fallback={<p class="loading-state">Loading assignment...</p>}>
        <Show when={assignment()}>
          {(current) => (
            <>
              <p class="eyebrow">Assignment overview</p>
              <h1>{current().title}</h1>
              <p class="page-lede">
                Work from the structures and concepts in front of you. Memorization is not the goal.
              </p>
              <dl class="assignment-facts">
                <div>
                  <dt>Questions per run</dt>
                  <dd>
                    {current().items.filter((item) => item.deliveryState === "active").length +
                      current().selectionGroups.reduce(
                        (count, group) => count + group.drawCount,
                        0,
                      )}
                  </dd>
                </div>
                <div>
                  <dt>Variation</dt>
                  <dd>Each attempt is a fresh variation.</dd>
                </div>
                <div>
                  <dt>Feedback</dt>
                  <dd>Feedback and scores are available according to the assignment settings.</dd>
                </div>
                <Show when={summary()}>
                  {(assignmentSummary) => (
                    <>
                      <div>
                        <dt>Score status</dt>
                        <dd role="status">{learnerProgressSummary(assignmentSummary())}</dd>
                      </div>
                      <Show when={assignmentSummary().scoreState === "available"}>
                        <div>
                          <dt>Current score</dt>
                          <dd>{learnerScoreValue(assignmentSummary().currentScore)}</dd>
                        </div>
                        <div>
                          <dt>Latest score</dt>
                          <dd>{learnerScoreValue(assignmentSummary().latestScore)}</dd>
                        </div>
                        <div>
                          <dt>Best score</dt>
                          <dd>{learnerScoreValue(assignmentSummary().bestScore)}</dd>
                        </div>
                      </Show>
                      <div>
                        <dt>Completed runs</dt>
                        <dd>{assignmentSummary().completedRunCount}</dd>
                      </div>
                      <div>
                        <dt>Total attempts</dt>
                        <dd>{assignmentSummary().totalQuestionAttempts}</dd>
                      </div>
                      <div>
                        <dt>Last activity</dt>
                        <dd>{formatActivity(assignmentSummary().lastActivityAt)}</dd>
                      </div>
                      <Show when={assignmentSummary().classStatistics}>
                        {(statistics) => (
                          <div>
                            <dt>Class statistics</dt>
                            <dd>{classStatisticsSummary(statistics())}</dd>
                          </div>
                        )}
                      </Show>
                    </>
                  )}
                </Show>
              </dl>
              <Show when={startError()}>
                {(message) => (
                  <p class="inline-error" role="alert">
                    {message()}
                  </p>
                )}
              </Show>
              <Show when={mayEdit() && course()}>
                {(currentCourse) => (
                  <A
                    class="quiet-action"
                    href={`/instructor/courses/${courseRouteReference(currentCourse().summary.reference)}/assignments/${assignmentRouteReference(current().reference)}/edit`}
                  >
                    Edit assignment
                  </A>
                )}
              </Show>
              <button
                class="primary-action wide-action"
                type="button"
                disabled={starting()}
                onClick={() => void startOrResume()}
              >
                {starting() ? "Opening practice..." : "Start or continue practice"}
              </button>
            </>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
