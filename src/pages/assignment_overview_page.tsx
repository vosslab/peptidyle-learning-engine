// assignment_overview_page.tsx - assignment entry and run-resume transition.

import { A, createAsync, useNavigate, useParams } from "@solidjs/router";
import { createSignal, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { resolveAssignmentRoute } from "../navigation/resolved_route";
import { runRouteReference } from "../navigation/public_route";
import { courseRouteReference, assignmentRouteReference } from "../navigation/public_route";
import { useSessionBootstrap } from "../auth/session_context";

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
  const course = createAsync(() => {
    const assignmentId = assignment()?.courseId;
    return assignmentId === undefined
      ? Promise.resolve(undefined)
      : runtime.queries.courseScope(assignmentId);
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
      navigate(`/runs/${runRouteReference(run.publicId)}`);
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
                  <dd>Each newly issued attempt receives a fresh seed.</dd>
                </div>
                <div>
                  <dt>Feedback</dt>
                  <dd>Released according to the assignment policy.</dd>
                </div>
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
                    href={`/instructor/courses/${courseRouteReference(currentCourse().summary.publicId)}/assignments/${assignmentRouteReference(current().publicId)}/edit`}
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
                {starting() ? "Opening practice..." : "Start or resume practice"}
              </button>
            </>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
