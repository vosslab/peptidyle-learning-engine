// Mounts the shipped daughter Course Blueprint update review and Assessment apply path.

import { type JSX } from "solid-js";
import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { render } from "solid-js/web";

import type { AssessmentBlueprintUpdateReview } from "../../src/api/assessment_release";
import type { LiveAssessmentWorkspace } from "../../src/api/assessment_release";
import type { CourseBlueprintUpdateReview } from "../../src/api/assessment_release";
import type { SaveBaseAssessmentPolicyInput } from "../../src/api/assessment_release";
import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { ContentClassificationItem } from "../../src/api/content_classification";
import type { CourseInstanceView } from "../../src/api/course_instance";
import { AssessmentWorkspaceLivePage } from "../../src/pages/assessment_workspace/assessment_workspace_live_page";
import { CourseInstancePage } from "../../src/pages/course_instance_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

const courseInstanceId = "CI7K3M2QAZ";
const assessmentId = "A8H4N6PA6";
const blueprintCourseId = "BP7K3M2QAF";
const disciplineUuid = "11111111-1111-4111-8111-111111111111";
const keptEntryId = "11111111-1111-4111-8111-444444444444";
const addedEntryId = "11111111-1111-4111-8111-555555555555";
const dueAt = "2026-10-15T09:00:00.000";

export interface BlueprintUpdateApplyCall {
  readonly courseInstanceId: string;
  readonly assessmentId: string;
  readonly expectedSourceBlueprintRevisionTuple: {
    readonly blueprintCourseId: string;
    readonly revisionNumber: string;
  };
  readonly expectedAssessmentEditNumber: string;
}

let disposeMount: (() => void) | undefined;
let durationWorkspace = daughterWorkspace(false);

function feedback(): LiveAssessmentWorkspace["studentFeedbackReleaseRule"] {
  return {
    score: "after_submit",
    per_item_correctness: "after_submit",
    submitted_response: "after_submit",
    question_answer: "never",
    question_answer_explanation: "never",
    class_statistics: "never",
    hints: "never",
    worked_solutions: "never",
  };
}

function activity(): LiveAssessmentWorkspace["activityRules"] {
  return {
    questionVariationRule: "newVariation",
    assessmentQuestionOrderRule: "authoredOrder",
  };
}

function daughterWorkspace(applied: boolean): LiveAssessmentWorkspace {
  const currentQuestion = {
    publishedQuestionId: "7K3M-79QP",
    revisionNumber: 1,
  };
  const addedQuestion = {
    publishedQuestionId: "2R5X-E7YA",
    revisionNumber: 1,
  };
  const currentEntry = {
    kind: "fixedQuestion" as const,
    id: keptEntryId,
    publishedQuestionRevisionTuple: currentQuestion,
    pointsPossible: "1",
    availability: "available" as const,
    scoringRule: "normal" as const,
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" as const },
  };
  const addedEntry = {
    ...currentEntry,
    id: addedEntryId,
    publishedQuestionRevisionTuple: addedQuestion,
  };
  return {
    id: assessmentId,
    assessmentEditNumber: applied ? "2" : "1",
    status: "unreleased",
    origin: {
      kind: "adopted",
      source: {
        blueprint_revision_tuple: {
          blueprintCourseId,
          revisionNumber: "1",
        },
        blueprint_assessment_id: "assessment-source",
      },
    },
    assessmentType: "regular_assignment",
    title: applied ? "Cell membranes" : "Membrane review",
    instructions: applied ? "Bring a labeled membrane diagram." : "",
    dueAt,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assessmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: activity(),
    studentFeedbackReleaseRule: feedback(),
    displayTimeZone: "America/Chicago",
    entries: applied ? [currentEntry, addedEntry] : [currentEntry],
    questions: [
      {
        publishedQuestionRevisionTuple: currentQuestion,
        questionTitle: "Current membrane question",
        description: "Current membrane question",
        bloom: null,
      },
      ...(applied
        ? [
            {
              publishedQuestionRevisionTuple: addedQuestion,
              questionTitle: "Membrane protein question",
              description: "Membrane protein question",
              bloom: null,
            },
          ]
        : []),
    ],
  };
}

function proposedReview(current: LiveAssessmentWorkspace): AssessmentBlueprintUpdateReview {
  const addedQuestion = {
    publishedQuestionId: "2R5X-E7YA",
    revisionNumber: 1,
  };
  return {
    assessment: current,
    sourceBlueprintRevisionTuple: {
      blueprintCourseId,
      revisionNumber: "3",
    },
    cannotApplyReason: null,
    proposed: {
      assessmentType: "regular_assignment",
      title: "Cell membranes",
      instructions: "Bring a labeled membrane diagram.",
      defaults: {
        assessment_attempt_time_limit_seconds: null,
        assessment_attempt_limit: null,
        late_work_rule: "reject",
        activity_rules: activity(),
        student_feedback_release_rule: feedback(),
      },
      entries: [
        {
          kind: "fixedQuestion",
          publishedQuestionRevisionTuple: {
            publishedQuestionId: "7K3M-79QP",
            revisionNumber: 1,
          },
          pointsPossible: "1",
          scoringRule: "normal",
          questionAttemptLimit: { maxAttempts: null },
          questionAttemptTimeLimit: { kind: "unlimited" },
        },
        {
          kind: "fixedQuestion",
          publishedQuestionRevisionTuple: addedQuestion,
          pointsPossible: "1",
          scoringRule: "normal",
          questionAttemptLimit: { maxAttempts: null },
          questionAttemptTimeLimit: { kind: "unlimited" },
        },
      ],
    },
  };
}

function queries(): ApplicationApi<OrdinaryBrowserApiClient>["queries"] {
  return {
    courseScope(): Promise<never> {
      return Promise.reject(new Error("This Blueprint update proof does not load a Course theme."));
    },
    assessmentAttemptScope(): Promise<never> {
      return Promise.reject(new Error("This Blueprint update proof does not load an Attempt."));
    },
    assessmentAttemptHistory(): Promise<never> {
      return Promise.reject(
        new Error("This Blueprint update proof does not load Attempt history."),
      );
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>["queries"];
}

function daughterCourse(): CourseInstanceView {
  return {
    courseInstance: {
      id: courseInstanceId,
      shortName: "Bio 101",
      longName: "Introductory Biology",
      lifecycleState: "active",
      courseEditNumber: "7",
      theme: "tundra",
      classification: {
        disciplineUuid,
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        tags: [],
      },
      term: { startDate: "2026-01-12", endDate: "2026-05-08" },
    },
    activeInstructorCount: 1,
    blueprintOrigin: {
      adoptedBlueprintRevisionTuple: { blueprintCourseId, revisionNumber: "1" },
      currentBlueprintRevisionTuple: { blueprintCourseId, revisionNumber: "3" },
    },
  };
}

function reviewPayload(): CourseBlueprintUpdateReview {
  return {
    adoptedBlueprintRevisionTuple: { blueprintCourseId, revisionNumber: "1" },
    currentBlueprintRevisionTuple: { blueprintCourseId, revisionNumber: "3" },
    assessments: [
      {
        assessmentId,
        title: "Cell membranes",
        assessmentType: "regular_assignment",
        matchesSource: false,
        cannotApplyReason: null,
      },
    ],
  };
}

function CourseHost(): JSX.Element {
  return <CourseInstancePage />;
}

function QuestionsHost(): JSX.Element {
  return <AssessmentWorkspaceLivePage section="questions" />;
}

function PoliciesHost(): JSX.Element {
  return <AssessmentWorkspaceLivePage section="policies" />;
}

/** Opens the daughter Course whose parent Blueprint is now a newer Revision. */
export function mountDaughterBlueprintReview(target: HTMLElement): void {
  disposeMount?.();
  const loads: string[] = [];
  (window as unknown as { courseBlueprintUpdateLoads: string[] }).courseBlueprintUpdateLoads =
    loads;
  const biology: ContentClassificationItem = {
    uuid: disciplineUuid,
    name: "Biology",
    isRetired: false,
  };
  const applicationApi = {
    client: {
      getCourseInstance: (): Promise<CourseInstanceView> => Promise.resolve(daughterCourse()),
      listCourseAssessments: () => Promise.resolve([]),
      getProfile: (): Promise<never> => new Promise(() => undefined),
      listDisciplinesIncludingRetired: () => Promise.resolve([biology]),
      listSubjects: () => Promise.resolve([]),
      listTopics: () => Promise.resolve([]),
      listSubtopics: () => Promise.resolve([]),
      getCourseBlueprintUpdateReview: (loadedCourseId: string) => {
        loads.push(loadedCourseId);
        return Promise.resolve(reviewPayload());
      },
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = `/instructor/courses/${courseInstanceId}`;
  const history = createMemoryHistory();
  history.set({ value: pathname });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route path="/instructor/courses/:courseInstanceId" component={CourseHost} />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}

/** Opens the adopted Assessment and applies the reviewed parent Blueprint Revision. */
export function mountDaughterBlueprintApply(target: HTMLElement): void {
  disposeMount?.();
  const applies: BlueprintUpdateApplyCall[] = [];
  (
    window as unknown as { blueprintUpdateApplies: BlueprintUpdateApplyCall[] }
  ).blueprintUpdateApplies = applies;
  let applied = false;
  const applicationApi = {
    client: {
      getLiveAssessmentWorkspace: () => Promise.resolve({ workspace: daughterWorkspace(applied) }),
      getAssessmentBlueprintUpdateReview: () =>
        Promise.resolve(proposedReview(daughterWorkspace(false))),
      applyAssessmentBlueprintUpdate: (
        loadedCourseId: string,
        loadedAssessmentId: string,
        input: {
          readonly expectedSourceBlueprintRevisionTuple: BlueprintUpdateApplyCall["expectedSourceBlueprintRevisionTuple"];
          readonly expectedAssessmentEditNumber: string;
        },
      ) => {
        applies.push({
          courseInstanceId: loadedCourseId,
          assessmentId: loadedAssessmentId,
          expectedSourceBlueprintRevisionTuple: input.expectedSourceBlueprintRevisionTuple,
          expectedAssessmentEditNumber: input.expectedAssessmentEditNumber,
        });
        applied = true;
        return Promise.resolve({ workspace: daughterWorkspace(true) });
      },
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/questions`;
  const history = createMemoryHistory();
  history.set({ value: pathname });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route
              path="/instructor/courses/:courseInstanceId/assessments/:assessmentId/questions"
              component={QuestionsHost}
            />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}

/** Opens Assessment Properties and keeps each saved duration on the next load. */
export function mountAssessmentDurationOverride(target: HTMLElement): void {
  disposeMount?.();
  const saves: Array<number | null> = [];
  (window as unknown as { assessmentDurationSaves: Array<number | null> }).assessmentDurationSaves =
    saves;
  const applicationApi = {
    client: {
      getLiveAssessmentWorkspace: () => Promise.resolve({ workspace: durationWorkspace }),
      saveBaseAssessmentPolicy: (
        _courseInstanceId: string,
        _assessmentId: string,
        input: SaveBaseAssessmentPolicyInput,
      ) => {
        saves.push(input.assessmentAttemptTimeLimitSeconds);
        durationWorkspace = {
          ...durationWorkspace,
          assessmentAttemptTimeLimitSeconds: input.assessmentAttemptTimeLimitSeconds,
        };
        return Promise.resolve({ workspace: durationWorkspace });
      },
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/properties`;
  const history = createMemoryHistory();
  history.set({ value: pathname });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route
              path="/instructor/courses/:courseInstanceId/assessments/:assessmentId/properties"
              component={PoliciesHost}
            />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}

/** Opens Assessment Properties and returns the three Question release issues. */
export function mountAssessmentReleaseQuestionIssues(target: HTMLElement): void {
  disposeMount?.();
  const applicationApi = {
    client: {
      getLiveAssessmentWorkspace: () => Promise.resolve({ workspace: daughterWorkspace(false) }),
      validateLiveAssessmentRelease: () =>
        Promise.resolve({
          canRelease: false,
          issues: ["noPublishedQuestions", "questionCountExceeded", "questionUnavailable"],
        }),
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/properties`;
  const history = createMemoryHistory();
  history.set({ value: pathname });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route
              path="/instructor/courses/:courseInstanceId/assessments/:assessmentId/properties"
              component={PoliciesHost}
            />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}

export interface AssessmentTimeSave {
  readonly courseInstanceId: string;
  readonly assessmentId: string;
  readonly rosterId: string;
  readonly timeMultiplier: number | null;
  readonly expectedAccommodationEditNumber: string | null;
}

/** Opens Assessment Properties so one active Student can receive 1.5X or 2X time. */
export function mountAssessmentStudentTime(target: HTMLElement): void {
  disposeMount?.();
  const saves: AssessmentTimeSave[] = [];
  (window as unknown as { assessmentTimeSaves: AssessmentTimeSave[] }).assessmentTimeSaves = saves;
  const rosterId = "AVERY";
  let timeMultiplier: number | null = null;
  let accommodationEditNumber: string | null = null;
  const applicationApi = {
    client: {
      getLiveAssessmentWorkspace: () => Promise.resolve({ workspace: daughterWorkspace(false) }),
      getLiveCourseRoster: () =>
        Promise.resolve([
          { rosterId, rosterName: "Avery Student", state: "activeStudent" as const },
          {
            rosterId: "INVITED",
            rosterName: "Invited Student",
            state: "invitationPending" as const,
          },
        ]),
      getAssessmentStudentTimeAccommodation: () =>
        Promise.resolve({
          rosterId,
          timeMultiplier,
          accommodationEditNumber,
          baseDurationSeconds: 120,
          effectiveDurationSeconds: 120,
          cappedAt24Hours: false,
        }),
      saveAssessmentStudentTimeAccommodation: (
        loadedCourseId: string,
        loadedAssessmentId: string,
        loadedRosterId: string,
        input: {
          readonly timeMultiplier: number | null;
          readonly expectedAccommodationEditNumber: string | null;
        },
      ) => {
        saves.push({
          courseInstanceId: loadedCourseId,
          assessmentId: loadedAssessmentId,
          rosterId: loadedRosterId,
          timeMultiplier: input.timeMultiplier,
          expectedAccommodationEditNumber: input.expectedAccommodationEditNumber,
        });
        timeMultiplier = input.timeMultiplier;
        accommodationEditNumber = timeMultiplier === null ? null : "1";
        return Promise.resolve({
          rosterId: loadedRosterId,
          timeMultiplier,
          accommodationEditNumber,
          baseDurationSeconds: 120,
          effectiveDurationSeconds: 120,
          cappedAt24Hours: false,
        });
      },
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/properties`;
  const history = createMemoryHistory();
  history.set({ value: pathname });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route
              path="/instructor/courses/:courseInstanceId/assessments/:assessmentId/properties"
              component={PoliciesHost}
            />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}

/** Opens Course Instance Assessment Properties for one adopted Assessment. */
export function mountCourseInstanceDelivery(target: HTMLElement): void {
  disposeMount?.();
  const applicationApi = {
    client: {
      getLiveAssessmentWorkspace: () => Promise.resolve({ workspace: daughterWorkspace(false) }),
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/properties`;
  const history = createMemoryHistory();
  history.set({ value: pathname });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route
              path="/instructor/courses/:courseInstanceId/assessments/:assessmentId/properties"
              component={PoliciesHost}
            />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}
