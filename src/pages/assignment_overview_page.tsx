// Student Assignment Access before the issued one-question attempt lane.

import { createAsync, useNavigate, useParams } from "@solidjs/router";
import { createEffect, createSignal, Match, Show, Switch, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import {
  assignmentAttemptRouteReference,
  parseAssignmentReference,
  parseCourseInstanceReference,
  type AssignmentRouteReference,
  type CourseInstanceRouteReference,
} from "../navigation/public_route";

function startDecisionMessage(decision: string): string {
  switch (decision) {
    case "may_start":
      return "This Assignment is available to start.";
    case "not_yet_available":
      return "This Assignment is not yet available.";
    case "closed":
      return "This Assignment is closed for new work.";
    case "attempt_limit_reached":
      return "The allowed number of Assignment Attempts has been reached.";
    case "late_work_refused":
      return "New work is not available under the late-work policy.";
    default:
      return "This Assignment is unavailable.";
  }
}

/** Public Course and Assignment References locate the view; the server re-authorizes each response. */
export function AssignmentOverviewPage(): JSX.Element {
  const runtime = useApplicationApi();
  const navigate = useNavigate();
  const params = useParams();
  const [starting, setStarting] = createSignal(false);
  const [startError, setStartError] = createSignal<string>();
  const course = (): CourseInstanceRouteReference | null =>
    parseCourseInstanceReference(params["courseRef"] ?? "");
  const assignment = (): AssignmentRouteReference | null =>
    parseAssignmentReference(params["assignmentRef"] ?? "");
  const access = createAsync(() => {
    const courseReference = course();
    const assignmentReference = assignment();
    if (courseReference === null || assignmentReference === null) return Promise.resolve(undefined);
    return runtime.client.getLiveAssignmentAccess(courseReference, assignmentReference);
  });
  createEffect(() => {
    const activeAssignmentAttempt = access()?.activeAssignmentAttempt;
    if (activeAssignmentAttempt === null || activeAssignmentAttempt === undefined) return;
    navigate(`/assignment-attempts/${assignmentAttemptRouteReference(activeAssignmentAttempt)}`, {
      replace: true,
    });
  });

  async function startAssignment(): Promise<void> {
    const courseReference = course();
    const assignmentReference = assignment();
    if (courseReference === null || assignmentReference === null || starting()) return;
    setStarting(true);
    setStartError(undefined);
    try {
      const attempt = await runtime.client.startLiveAssignment(
        courseReference,
        assignmentReference,
      );
      const attemptReference = assignmentAttemptRouteReference(attempt.assignmentAttempt);
      navigate(`/assignment-attempts/${attemptReference}`, { replace: true });
    } catch (_error: unknown) {
      setStartError("Assignment could not be started. Please try again.");
    } finally {
      setStarting(false);
    }
  }

  return (
    <section class="page" data-route-surface="assignmentOverview">
      <h1>Assignment</h1>
      <Show when={access()} fallback={<p class="loading-state">Loading Assignment Access...</p>}>
        {(current) => (
          <>
            <Show
              when={current().activeAssignmentAttempt === null}
              fallback={
                <p class="loading-state" role="status">
                  Resuming Assignment...
                </p>
              }
            >
              <p role="status">{startDecisionMessage(current().startDecision)}</p>
              <Switch>
                <Match when={current().startDecision === "may_start"}>
                  <button
                    class="primary-action"
                    type="button"
                    disabled={starting()}
                    onClick={() => void startAssignment()}
                  >
                    {starting() ? "Starting Assignment..." : "Start Assignment"}
                  </button>
                </Match>
                <Match when={true}>
                  <p>Check with your Instructor if you expected this Assignment to be available.</p>
                </Match>
              </Switch>
              <Show when={startError()}>
                {(message) => (
                  <p role="alert" class="inline-error">
                    {message()}
                  </p>
                )}
              </Show>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
