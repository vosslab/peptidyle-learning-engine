// Student Assessment Access before the issued one-question attempt lane.

import { useNavigate, useParams } from "@solidjs/router";
import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  Match,
  on,
  onCleanup,
  Show,
  Switch,
  type JSX,
} from "solid-js";

import { assessmentTypePresentation } from "../assessment_type_presentation";
import type {
  LiveAssessmentAccess,
  LiveAssessmentPreviousAttempt,
} from "../api/assessment_attempt_issuance";
import { useApplicationApi } from "../api/application_api";
import { StudentAssessmentStartFacts } from "../components/student_assessment_presentation";
import { PageFrame } from "../components/page_frame/page_frame";
import type { RecordContent } from "../components/data_sheet/record_content";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import {
  assessmentAttemptRouteId,
  parseAssessmentId,
  parseCourseInstanceId,
  type AssessmentRouteId,
  type CourseInstanceRouteId,
} from "../navigation/public_route";
import {
  assessmentAttemptPath,
  assessmentAttemptRouteState,
} from "../navigation/assessment_attempt_route";
import { formatPointScore } from "../score_format";
import {
  previousAttemptSearchFields,
  type PreviousAttemptSortKey,
} from "./assessment_previous_attempt_search_fields";

function previousAttemptContent(
  attempt: LiveAssessmentPreviousAttempt,
  courseInstanceId: CourseInstanceRouteId,
  navigate: ReturnType<typeof useNavigate>,
): RecordContent {
  return {
    title: `Attempt ${attempt.attemptNumber}`,
    details: [
      {
        kind: "text",
        label: "State",
        value: attempt.state === "submitted" ? "Submitted" : "Closed",
      },
      {
        kind: "text",
        label: "Score",
        value:
          attempt.score === undefined
            ? "Score pending"
            : `${formatPointScore(attempt.score.pointsEarned, attempt.score.pointsPossible)} points`,
      },
    ],
    actions: [
      {
        id: "review-attempt",
        kind: "command",
        label: "Review Attempt",
        onClick: () =>
          navigate(assessmentAttemptPath("review", courseInstanceId), {
            state: assessmentAttemptRouteState(
              assessmentAttemptRouteId(attempt.assessmentAttemptId),
            ),
          }),
      },
    ],
  };
}

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
  const [accessSource, { refetch: refetchAccess }] = createResource(
    () => {
      const courseInstanceId = course();
      const assessmentId = assessment();
      return courseInstanceId === null || assessmentId === null
        ? undefined
        : { courseInstanceId, assessmentId };
    },
    async ({ courseInstanceId, assessmentId }) => ({
      courseInstanceId,
      assessmentId,
      value: await runtime.client.getLiveAssessmentAccess(courseInstanceId, assessmentId),
    }),
  );
  const access = (): LiveAssessmentAccess | undefined => {
    const loaded = accessSource.latest;
    return loaded?.courseInstanceId === course() && loaded?.assessmentId === assessment()
      ? loaded.value
      : undefined;
  };
  const previousAttempts = createMemo(() => access()?.previousAttempts);
  const previousAttemptSearch = createLocalCompleteSearch(
    {
      rows: previousAttempts,
      loading: () => accessSource.loading,
      error: (): unknown => accessSource.error,
      refetchSource: refetchAccess,
    },
    {
      fields: previousAttemptSearchFields,
      rowId: (attempt) => attempt.assessmentAttemptId,
      keyword: (attempt) => `Attempt ${attempt.attemptNumber} ${attempt.state}`,
      compare: (key: PreviousAttemptSortKey, left, right) =>
        key === "attemptNumber" ? left.attemptNumber - right.attemptNumber : undefined,
      content: (attempt) => previousAttemptContent(attempt, course()!, navigate),
    },
  );
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
      navigate(assessmentAttemptPath("attempt", courseInstanceId), {
        replace: true,
        state: assessmentAttemptRouteState(assessmentAttemptId),
      });
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
    <PageFrame routeSurface="assessmentOverview">
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
            <SearchBlock
              layout="above"
              controls={
                <SearchControls
                  state={previousAttemptSearch.state}
                  textLabel="Search previous Attempts"
                />
              }
              state={previousAttemptSearch.state}
              collection={{
                ariaLabel: "Previous Attempts",
                fields: previousAttemptSearchFields,
                query: previousAttemptSearch.query,
                get emptyState() {
                  return {
                    title:
                      previousAttemptSearch.state.query().text.length > 0 ||
                      previousAttemptSearch.state.query().filters.size > 0
                        ? "No previous Attempts match these filters"
                        : "No previous Attempts",
                  };
                },
              }}
            />
          </>
        )}
      </Show>
    </PageFrame>
  );
}
