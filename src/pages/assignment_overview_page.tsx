// assignment_overview_page.tsx - assignment entry and Assignment Attempt transition.

import { createAsync, useNavigate, useParams } from "@solidjs/router";
import { createSignal, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { resolveAssignmentRoute } from "../navigation/resolved_route";
import { assignmentAttemptRouteReference } from "../navigation/public_route";
import {
  StudentAssignmentPresentation,
  toStudentAssignmentPresentationData,
} from "../components/student_assignment_presentation";
import "../components/student_assignment_presentation.css";

export function AssignmentOverviewPage(): JSX.Element {
  const runtime = useApiRuntime();
  const navigate = useNavigate();
  const params = useParams();
  const [starting, setStarting] = createSignal(false);
  const [startError, setStartError] = createSignal<string>();
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
  async function startOrResume(): Promise<void> {
    const assignmentId = assignment()?.id;
    const courseId = course()?.summary.id;
    if (courseId === undefined || assignmentId === undefined || starting()) {
      return;
    }
    setStarting(true);
    setStartError(undefined);
    try {
      const assignmentAttempt = await runtime.client.startAssignmentAttempt(courseId, assignmentId);
      navigate(
        `/assignment-attempts/${assignmentAttemptRouteReference(assignmentAttempt.reference)}`,
      );
    } catch (error: unknown) {
      setStartError(
        error instanceof Error
          ? `Could not open the Assignment Attempt: ${error.message}. Try again.`
          : "Could not open the Assignment Attempt. Try again.",
      );
      setStarting(false);
    }
  }

  return (
    <section class="page" data-route-surface="assignmentOverview">
      <Suspense fallback={<p class="loading-state">Loading assignment...</p>}>
        <Show when={assignment()}>
          {(current) => (
            <StudentAssignmentPresentation
              assignment={toStudentAssignmentPresentationData(current())}
              progress={summary()}
              primaryAction={
                <>
                  <Show when={startError()}>
                    {(message) => (
                      <p class="inline-error" role="alert">
                        {message()}
                      </p>
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
              }
            />
          )}
        </Show>
      </Suspense>
    </section>
  );
}
