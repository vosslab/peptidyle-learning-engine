// Student Assessment Access before the issued one-question attempt lane.

import { A, createAsync, useNavigate, useParams } from "@solidjs/router";
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  on,
  onCleanup,
  Show,
  Switch,
  type JSX,
} from "solid-js";

import { assessmentTypePresentation } from "../assessment_type_presentation";
import { useApplicationApi } from "../api/application_api";
import { StudentAssessmentStartFacts } from "../components/student_assessment_presentation";
import { PageFrame } from "../components/page_frame";
import {
  assessmentAttemptRouteId,
  parseAssessmentId,
  parseCourseInstanceId,
  type AssessmentRouteId,
  type CourseInstanceRouteId,
} from "../navigation/public_route";
import { RibbonIcon } from "../ribbon/ribbon_icon";

/** Public Course Instance and Assessment IDs locate the view; the server re-authorizes each response. */
export function AssessmentOverviewPage(): JSX.Element {
  const runtime = useApplicationApi();
  const navigate = useNavigate();
  const params = useParams();
  const [starting, setStarting] = createSignal(false);
  const [startError, setStartError] = createSignal<string>();
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });
  const course = (): CourseInstanceRouteId | null =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
  const assessment = (): AssessmentRouteId | null =>
    parseAssessmentId(params["assessmentId"] ?? "");
  const access = createAsync(() => {
    const courseInstanceId = course();
    const assessmentId = assessment();
    if (courseInstanceId === null || assessmentId === null) return Promise.resolve(undefined);
    return runtime.client.getLiveAssessmentAccess(courseInstanceId, assessmentId);
  });
  const activeAttemptId = createMemo(() => {
    const activeAttempt = access()?.activeAssessmentAttemptId;
    return activeAttempt === null || activeAttempt === undefined
      ? null
      : assessmentAttemptRouteId(activeAttempt);
  });
  createEffect(
    on(activeAttemptId, (attemptId) => {
      // Only a changed active Assessment Attempt ID triggers automatic Resume, never busy/error updates.
      if (attemptId !== null) void startAssessment();
    }),
  );

  async function startAssessment(): Promise<void> {
    const courseInstanceId = course();
    const assessmentId = assessment();
    const assessmentType = access()?.assessmentType;
    const activeAttempt = access()?.activeAssessmentAttemptId;
    const resuming = activeAttempt !== null && activeAttempt !== undefined;
    if (
      courseInstanceId === null ||
      assessmentId === null ||
      assessmentType === undefined ||
      disposed ||
      starting()
    ) {
      return;
    }
    setStarting(true);
    setStartError(undefined);
    const requestIsCurrent = (): boolean =>
      !disposed && course() === courseInstanceId && assessment() === assessmentId;
    try {
      // ASVS 2.3.1: finish authorized same-Attempt issuance before reading progress.
      const attempt = await runtime.client.startLiveAssessment(courseInstanceId, assessmentId);
      if (!requestIsCurrent()) return;
      const assessmentAttemptId = assessmentAttemptRouteId(attempt.assessmentAttemptId);
      navigate(`/assessment-attempts/${assessmentAttemptId}`, { replace: true });
    } catch (_error: unknown) {
      if (!requestIsCurrent()) return;
      setStartError(
        `${assessmentTypePresentation(assessmentType).label} could not be ${resuming ? "resumed" : "started"}. Please try again.`,
      );
    } finally {
      if (!disposed) setStarting(false);
    }
  }

  return (
    <PageFrame
      routeSurface="assessmentOverview"
      title={access()?.title ?? "Assessment"}
      eyebrow={
        access() === undefined ? (
          "Assessment"
        ) : (
          <>
            <RibbonIcon glyph={assessmentTypePresentation(access()!.assessmentType).icon} />
            <span>{assessmentTypePresentation(access()!.assessmentType).label}</span>
          </>
        )
      }
    >
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
            <StudentAssessmentStartFacts
              questionCount={current().questionCount}
              pointsPossible={current().pointsPossible}
              timeLimitSeconds={current().decision.timeLimitSeconds}
              decision={current().decision}
            />
            <section
              class="student-assessment-action-region"
              aria-label={`${assessmentTypePresentation(current().assessmentType).label} access`}
            >
              <div class="student-assessment-primary-action">
                <Switch>
                  <Match when={current().activeAssessmentAttemptId !== null}>
                    <button
                      class="primary-action wide-action"
                      type="button"
                      disabled={starting()}
                      onClick={() => void startAssessment()}
                    >
                      {starting()
                        ? `Resuming ${assessmentTypePresentation(current().assessmentType).label}...`
                        : `Resume ${assessmentTypePresentation(current().assessmentType).label}`}
                    </button>
                  </Match>
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
                      {assessmentTypePresentation(current().assessmentType).label} to be available.
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
            <Show when={current().previousAttempts.length > 0}>
              <section class="assessment-history" aria-labelledby="assessment-history-heading">
                <h2 id="assessment-history-heading">Previous attempts</h2>
                <ul class="assessment-history__list">
                  <For each={current().previousAttempts}>
                    {(attempt) => (
                      <li>
                        <A
                          href={`/assessment-attempts/${assessmentAttemptRouteId(attempt.assessmentAttemptId)}/summary`}
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
    </PageFrame>
  );
}
