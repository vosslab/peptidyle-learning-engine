// Student Assessment Access before the issued one-question attempt lane.

import { A, createAsync, useNavigate, useParams } from "@solidjs/router";
import { createEffect, createSignal, For, Match, Show, Switch, type JSX } from "solid-js";

import { assessmentTypePresentation } from "../assessment_type_presentation";
import { useApplicationApi } from "../api/application_api";
import { StudentAssessmentStartFacts } from "../components/student_assessment_presentation";
import {
  assessmentAttemptRouteReference,
  parseAssessmentReference,
  parseCourseInstanceReference,
  type AssessmentRouteReference,
  type CourseInstanceRouteReference,
} from "../navigation/public_route";
import { RibbonIcon } from "../ribbon/ribbon_icon";

/** Public Course and Assessment References locate the view; the server re-authorizes each response. */
export function AssessmentOverviewPage(): JSX.Element {
  const runtime = useApplicationApi();
  const navigate = useNavigate();
  const params = useParams();
  const [starting, setStarting] = createSignal(false);
  const [startError, setStartError] = createSignal<string>();
  const course = (): CourseInstanceRouteReference | null =>
    parseCourseInstanceReference(params["courseRef"] ?? "");
  const assessment = (): AssessmentRouteReference | null =>
    parseAssessmentReference(params["assessmentRef"] ?? "");
  const access = createAsync(() => {
    const courseReference = course();
    const assessmentReference = assessment();
    if (courseReference === null || assessmentReference === null) return Promise.resolve(undefined);
    return runtime.client.getLiveAssessmentAccess(courseReference, assessmentReference);
  });
  createEffect(() => {
    const activeAssessmentAttempt = access()?.activeAssessmentAttempt;
    if (activeAssessmentAttempt === null || activeAssessmentAttempt === undefined) return;
    navigate(`/assessment-attempts/${assessmentAttemptRouteReference(activeAssessmentAttempt)}`, {
      replace: true,
    });
  });

  async function startAssessment(): Promise<void> {
    const courseReference = course();
    const assessmentReference = assessment();
    if (courseReference === null || assessmentReference === null || starting()) return;
    setStarting(true);
    setStartError(undefined);
    try {
      const attempt = await runtime.client.startLiveAssessment(
        courseReference,
        assessmentReference,
      );
      const attemptReference = assessmentAttemptRouteReference(attempt.assessmentAttempt);
      navigate(`/assessment-attempts/${attemptReference}`, { replace: true });
    } catch (_error: unknown) {
      setStartError("Assessment could not be started. Please try again.");
    } finally {
      setStarting(false);
    }
  }

  return (
    <section class="page" data-route-surface="assessmentOverview">
      <Show
        when={access()}
        fallback={
          <p class="loading-state" role="status">
            Loading...
          </p>
        }
      >
        {(current) => (
          <>
            <p class="eyebrow">
              <RibbonIcon glyph={assessmentTypePresentation(current().assessmentType).icon} />
              <span>{assessmentTypePresentation(current().assessmentType).label}</span>
            </p>
            <h1>{current().title}</h1>
            <StudentAssessmentStartFacts
              questionCount={current().questionCount}
              pointsPossible={current().pointsPossible}
              timeLimitSeconds={current().decision.timeLimitSeconds}
              decision={current().decision}
            />
            <Show
              when={current().activeAssessmentAttempt === null}
              fallback={
                <p class="loading-state" role="status">
                  Resuming {assessmentTypePresentation(current().assessmentType).label}...
                </p>
              }
            >
              <section
                class="student-assessment-action-region"
                aria-label={`${assessmentTypePresentation(current().assessmentType).label} access`}
              >
                <div class="student-assessment-primary-action">
                  <Switch>
                    <Match when={current().decision.startDecision === "may_start"}>
                      <button
                        class="primary-action wide-action"
                        type="button"
                        disabled={starting()}
                        onClick={() => void startAssessment()}
                      >
                        {starting()
                          ? `Starting ${assessmentTypePresentation(current().assessmentType).label}...`
                          : `Start ${assessmentTypePresentation(current().assessmentType).label}`}
                      </button>
                    </Match>
                    <Match when={true}>
                      <p>
                        Check with your Instructor if you expected this{" "}
                        {assessmentTypePresentation(current().assessmentType).label} to be
                        available.
                      </p>
                    </Match>
                  </Switch>
                  <Show when={startError()}>
                    {(message) => (
                      <p role="alert" class="inline-error">
                        {message()}
                      </p>
                    )}
                  </Show>
                </div>
              </section>
            </Show>
            <Show when={current().previousAttempts.length > 0}>
              <section class="assessment-history" aria-labelledby="assessment-history-heading">
                <h2 id="assessment-history-heading">Previous attempts</h2>
                <ul class="assessment-history__list">
                  <For each={current().previousAttempts}>
                    {(attempt) => (
                      <li>
                        <A
                          href={`/assessment-attempts/${assessmentAttemptRouteReference(attempt.assessmentAttempt)}/summary`}
                        >
                          Attempt {attempt.attemptNumber}
                        </A>
                        <span>{attempt.state === "submitted" ? "Submitted" : "Closed"}</span>
                        <span>
                          {attempt.score === undefined
                            ? "Score is not available."
                            : `${attempt.score.pointsEarned} of ${attempt.score.pointsPossible} points`}
                        </span>
                      </li>
                    )}
                  </For>
                </ul>
              </section>
            </Show>
            <Show when={current().previousAttempts.length === 0}>
              <p class="empty-state" role="note">
                You do not have a previous attempt for this{" "}
                {assessmentTypePresentation(current().assessmentType).label}.
              </p>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
