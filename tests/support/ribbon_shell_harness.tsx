// ribbon_shell_harness.tsx - current-source App composition for compiled-harness evidence.

import { createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";
import { MemoryRouter, createMemoryHistory, useLocation } from "@solidjs/router";

import "../../src/browser_environment";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import { App } from "../../src/app";
import { ApplicationShell } from "../../src/application_shell";
import { SessionProvider } from "../../src/auth/session_context";
import { PageFrame } from "../../src/components/page_frame";
import { useAppearance } from "../../src/appearance/appearance_context";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { CourseInstanceSummary, CourseInstanceView } from "../../src/api/course_instance";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { ProfileAvatarView } from "../../src/api/profile_avatar";
import type { ProfileSettings } from "../../src/api/profile_settings";
import type { CourseAssessmentSummary } from "../../src/api/assessment_release";
import type { CourseGradebook } from "../../src/api/live_gradebook";
import type { CourseRosterEntry } from "../../src/api/course_roster";
import type {
  AuthenticatedSession,
  CourseRouteView,
  CourseSummary,
  CursorPage,
} from "../../src/api/contracts";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { UserRole } from "../../generated/api/UserRole";
import { routeContractForPathname, type RouteId } from "../../src/route_contract";
import { InstalledWasmFacadeProvider } from "../../src/wasm/context";
import type {
  CapabilityValidator,
  ResponseFormatValidator,
  TimerEvaluator,
  WasmFacade,
} from "../../src/wasm";
// prettier-ignore
import type {
  StudentAssessmentLandingSummary,
} from "../../generated/api/StudentAssessmentLandingSummary";
import { appRoutes, notFoundRoute } from "../../src/routes";
import type {
  RibbonControlModel,
  RibbonModel,
  RibbonTaskAreaModel,
} from "../../src/ribbon/ribbon_contract";
import type { RibbonDestinationId } from "../../src/ribbon/ribbon_catalog";
import { assignmentAttemptContext, courseRouteData } from "./route_scope_provider_fixtures";
import {
  FAST_UI_QUESTION_LIBRARY_SEED,
  fastUiQuestionLibraryPage,
} from "./fast_ui_question_library_fixture";
import { materializeRibbonRoute, M6_RIBBON_FIXTURES } from "./ribbon_model_fixtures";

interface QueryFunction<Arguments extends ReadonlyArray<unknown>, Result> {
  (...arguments_: Arguments): Promise<Result>;
  readonly key: string;
  readonly keyFor: (...arguments_: Arguments) => string;
}

function queryFunction<Arguments extends ReadonlyArray<unknown>, Result>(
  key: string,
  resolve: (...arguments_: Arguments) => Promise<Result>,
): QueryFunction<Arguments, Result> {
  const callable = Object.assign(resolve, {
    key,
    keyFor: (...arguments_: Arguments) => `${key}:${JSON.stringify(arguments_)}`,
  });
  return callable;
}

interface DeferredCourseScopes {
  readonly release: (courseInstanceId: string) => void;
  readonly requestCount: (courseInstanceId: string) => number;
  readonly waitForRelease: (courseInstanceId: string) => Promise<void>;
}

interface DeferredSession {
  readonly release: () => void;
  readonly waitForSession: () => Promise<AuthenticatedSession>;
}

const FIXTURE_CLASSIFICATION = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
} satisfies CourseClassification;

function deferredSession(): DeferredSession {
  let resolveSession: (() => void) | undefined;
  const sessionReady = new Promise<void>((resolve) => {
    resolveSession = resolve;
  });
  function release(): void {
    if (resolveSession === undefined) {
      throw new Error("Application-shell evidence released the session more than once.");
    }
    const resolve = resolveSession;
    resolveSession = undefined;
    resolve();
  }
  function waitForSession(): Promise<AuthenticatedSession> {
    return sessionReady.then(instructorSession);
  }
  return { release, waitForSession };
}

interface PresentationQueryCounts {
  readonly assessments: () => number;
  readonly courseInstances: () => number;
  readonly revokedRosterIds: () => readonly string[];
}

function deferredCourseScopes(): DeferredCourseScopes {
  const requests = new Map<string, number>();
  const releases = new Map<string, () => void>();

  function release(courseInstanceId: string): void {
    const resolve = releases.get(courseInstanceId);
    if (resolve === undefined) {
      throw new Error(
        `Application-shell evidence cannot release an unrequested course scope: ${courseInstanceId}.`,
      );
    }
    releases.delete(courseInstanceId);
    resolve();
  }

  function waitForRelease(courseInstanceId: string): Promise<void> {
    requests.set(courseInstanceId, (requests.get(courseInstanceId) ?? 0) + 1);
    if (courseInstanceId !== "CI7K3M2QAZ") return Promise.resolve();
    return new Promise((resolve) => {
      releases.set(courseInstanceId, resolve);
    });
  }

  return {
    release,
    requestCount: (courseInstanceId) => requests.get(courseInstanceId) ?? 0,
    waitForRelease,
  };
}

function inspectionQuestionDetails(): QuestionDetails {
  const publishedQuestionId = "7K3M-79QP";
  const publishedQuestionRevisionTuple = {
    publishedQuestionId,
    revisionNumber: 1,
  };
  return {
    summary: {
      questionId: publishedQuestionId,
      publishedQuestionRevisionTuple,
      backend: "ple",
      questionFormat: "pleQuestionJson",
      questionType: "fillInBlank",
      capabilities: [],
      metadata: {
        questionTitle: "Peptide bond",
        questionDescription: "Instructor note about the peptide-bond prompt.",
        tags: [],
        questionLicense: null,
        questionCitation: null,
        language: "en",
      },
      authorship: { authors: [{ displayName: "Ada Instructor", accountId: null }] },
      availability: { availability: "available" },
      publishedAt: 1_700_000_000_000,
      bloom: null,
    },
    disciplineName: "Biology",
    subjectName: "Biochemistry",
    disciplineIsRetired: false,
    prompt: {
      kind: "static",
      blocks: [{ kind: "text", markdown: "Which atoms form a peptide bond?" }],
    },
    responsePreview: { kind: "shortText" },
    evidence: { state: "unavailable" },
    usage: {
      summary: {
        globalCourseCount: 0,
        globalAssessmentCount: 0,
        ownCourseCount: 0,
        ownAssessmentCount: 0,
      },
      ownCourses: [],
      ownCoursesTruncated: false,
    },
  };
}

function presentationApi(deferredScopes?: DeferredCourseScopes): {
  readonly api: ApplicationApi<OrdinaryBrowserApiClient>;
  readonly counts: PresentationQueryCounts;
  readonly holdLiveCourseRoster: () => void;
  readonly releaseLiveCourseRoster: () => void;
  readonly failNextRosterRevoke: () => void;
  readonly clearStudentViewTransport: () => void;
  readonly studentViewTransport: () => readonly string[];
  readonly seedAuthoringClassification: () => void;
  readonly vocabularyWrites: () => readonly string[];
} {
  const courses: CursorPage<CourseSummary> = { items: [], nextCursor: null };
  const assessments: CursorPage<StudentAssessmentLandingSummary> = {
    items: [],
    nextCursor: null,
  };
  let assessmentQueries = 0;
  let courseInstanceQueries = 0;
  const revokedRosterIds: string[] = [];
  const rosterRows: ReadonlyArray<CourseRosterEntry> = [
    { rosterId: "RU-001", rosterName: "Avery Thompson", state: "activeStudent" },
    { rosterId: "RU-002", rosterName: "Morgan Lee", state: "invitationPending" },
  ];
  let holdRoster = false;
  let releaseRoster: (() => void) | undefined;
  let failNextRevoke = false;
  const studentViewTransport: string[] = [];
  const disciplines: Array<{ uuid: string; name: string; isRetired: boolean }> = [];
  const subjects: Array<{ uuid: string; name: string; isRetired: boolean }> = [];
  const topics: Array<{ uuid: string; name: string; isRetired: boolean }> = [];
  const subtopics: Array<{ uuid: string; name: string; isRetired: boolean }> = [];
  const vocabularyWrites: string[] = [];
  let createdVocabularyCount = 0;
  const createdVocabularyUuids = [
    "33333333-3333-4333-8333-333333333333",
    "44444444-4444-4444-8444-444444444444",
    "55555555-5555-4555-8555-555555555555",
  ];
  function nextCreatedVocabularyUuid(): string {
    const uuid = createdVocabularyUuids[createdVocabularyCount];
    createdVocabularyCount += 1;
    if (uuid === undefined) {
      throw new Error("Authoring classification evidence ran out of vocabulary ids.");
    }
    return uuid;
  }
  function seedAuthoringClassification(): void {
    disciplines.length = 0;
    subjects.length = 0;
    topics.length = 0;
    subtopics.length = 0;
    vocabularyWrites.length = 0;
    createdVocabularyCount = 0;
    disciplines.push({
      uuid: "11111111-1111-4111-8111-111111111111",
      name: "Biology",
      isRetired: false,
    });
    vocabularyWrites.push("seed:11111111-1111-4111-8111-111111111111");
  }
  const studentViewQuestion = {
    publishedQuestionId: "7K3M-79QP",
    revisionNumber: 1,
  };
  const queries = {
    courses: queryFunction("courses", () => Promise.resolve(courses)),
    assessments: queryFunction("course-assessments", (_courseId: CourseInstanceId) => {
      assessmentQueries += 1;
      return Promise.resolve(assessments);
    }),
    resolveCourse: queryFunction("resolve-course", (courseInstanceId: string) =>
      Promise.resolve({ courseId: `course-${courseInstanceId}` }),
    ),
    courseScope: queryFunction("course-scope", (courseId: string) => {
      const courseInstanceId = courseId.replace("course-", "");
      const released =
        deferredScopes === undefined
          ? Promise.resolve()
          : deferredScopes.waitForRelease(courseInstanceId);
      return released.then(() => instructorCourseRouteData(courseInstanceId));
    }),
    assessmentAttemptScope: queryFunction("assessment-attempt-scope", () =>
      Promise.resolve(assignmentAttemptContext("CI7K3M2QAZ")),
    ),
    assessmentAttemptHistory: queryFunction("assessment-attempt-history", () =>
      Promise.reject(
        new Error("Application-shell evidence does not enter attempt-history content."),
      ),
    ),
    questionDetails: queryFunction("question-details", () =>
      Promise.resolve(inspectionQuestionDetails()),
    ),
  };
  const client = {
    getProfile: (): Promise<ProfileSettings> =>
      Promise.resolve({
        timeZone: "America/Chicago",
        displayModePreference: null,
        personalTheme: "grass",
      }),
    getProfileAvatar: (): Promise<ProfileAvatarView> => Promise.resolve({ avatar: null }),
    getAccountSettings: (): Promise<ProfileSettings> =>
      Promise.resolve({
        timeZone: "America/Chicago",
        displayModePreference: null,
        personalTheme: "grass",
      }),
    listCourseInstances: (): Promise<ReadonlyArray<CourseInstanceSummary>> =>
      Promise.resolve([activeFixtureCourse("CI7K3M2QAZ")]),
    listInstallationCourses: (): Promise<unknown> =>
      Promise.resolve({ courses: [], nextCursor: null }),
    loadInstallationCourse: (): Promise<unknown> =>
      Promise.reject(new Error("installation Course inspection is not mounted in ribbon evidence")),
    listDisciplinesIncludingRetired: (): Promise<unknown> => Promise.resolve(disciplines.slice()),
    listSubjects: (_disciplineUuid: string): Promise<unknown> => Promise.resolve(subjects.slice()),
    listTopics: (_subjectUuid: string): Promise<unknown> => Promise.resolve(topics.slice()),
    listSubtopics: (_topicUuid: string): Promise<unknown> => Promise.resolve(subtopics.slice()),
    createSubject: (name: string, _disciplineUuid: string): Promise<unknown> => {
      const trimmed = name.trim();
      vocabularyWrites.push(`createSubject:${trimmed}`);
      if (trimmed === "Existing Biochemistry") {
        return Promise.resolve({
          uuid: "22222222-2222-4222-8222-222222222222",
          name: trimmed,
          needsAcceptance: true,
        });
      }
      const item = { uuid: nextCreatedVocabularyUuid(), name: trimmed, isRetired: false };
      subjects.push(item);
      return Promise.resolve({ uuid: item.uuid, name: item.name, needsAcceptance: false });
    },
    acceptSubjectDiscipline: (uuid: string, _disciplineUuid: string): Promise<unknown> => {
      vocabularyWrites.push(`acceptSubject:${uuid}`);
      const item = { uuid, name: "Existing Biochemistry", isRetired: false };
      subjects.push(item);
      return Promise.resolve(item);
    },
    createTopic: (name: string, _subjectUuid: string): Promise<unknown> => {
      const trimmed = name.trim();
      vocabularyWrites.push(`createTopic:${trimmed}`);
      const item = { uuid: nextCreatedVocabularyUuid(), name: trimmed, isRetired: false };
      topics.push(item);
      return Promise.resolve(item);
    },
    createSubtopic: (name: string, _topicUuid: string): Promise<unknown> => {
      const trimmed = name.trim();
      vocabularyWrites.push(`createSubtopic:${trimmed}`);
      const item = { uuid: nextCreatedVocabularyUuid(), name: trimmed, isRetired: false };
      subtopics.push(item);
      return Promise.resolve(item);
    },
    listBlueprintCourses: (): Promise<CursorPage<BlueprintCourseSummaryView>> =>
      Promise.resolve({ items: [], nextCursor: null }),
    getCourseInstance: (
      courseInstanceId: CourseInstanceView["courseInstance"]["id"],
    ): Promise<CourseInstanceView> => {
      courseInstanceQueries += 1;
      return Promise.resolve({
        courseInstance: activeFixtureCourse(courseInstanceId),
        activeInstructorCount: 1,
        blueprintOrigin: null,
      });
    },
    listCourseAssessments: (): Promise<ReadonlyArray<CourseAssessmentSummary>> => {
      assessmentQueries += 1;
      return Promise.resolve([]);
    },
    getCourseGradebook: (courseInstanceId: string): Promise<CourseGradebook> =>
      Promise.resolve({
        courseInstanceId,
        studentWork: [
          {
            rosterId: "RU-001",
            rosterName: "Avery Thompson",
            assessmentId: "A9D2RX5AF",
            assessmentTitle: "Protein structure practice",
            assessmentAttemptCompletion: "inProgress",
            expiredSubmitting: false,
            score: { pointsEarned: 18, pointsPossible: 20 },
          },
        ],
      }),
    getLiveCourseRoster: (): Promise<ReadonlyArray<CourseRosterEntry>> => {
      if (!holdRoster) return Promise.resolve(rosterRows);
      return new Promise((resolve) => {
        releaseRoster = (): void => resolve(rosterRows);
      });
    },
    revokeLiveCourseRosterEntry: (_courseInstanceId: string, rosterId: string): Promise<void> => {
      if (failNextRevoke) {
        failNextRevoke = false;
        return Promise.reject(new Error("roster change was refused"));
      }
      revokedRosterIds.push(rosterId);
      return Promise.resolve();
    },
    getLiveAssessmentWorkspace: (): Promise<{
      workspace: { title: string; assessmentEditNumber: string };
    }> => {
      studentViewTransport.push("getLiveAssessmentWorkspace");
      return Promise.resolve({
        workspace: {
          title: "Protein structure practice",
          assessmentEditNumber: "1",
        },
      });
    },
    getInstructorStudentView: (): Promise<{
      assessmentEditNumber: string;
      status: "released";
      title: string;
      instructions: string;
      displayTimeZone: string;
      delivery: {
        available_at: null;
        due_at: null;
        closes_at: null;
        assessment_attempt_time_limit_seconds: null;
        attempt_limit: null;
        late_work_rule: "reject";
      };
      entries: ReadonlyArray<{
        kind: "presented";
        authoredPosition: number;
        questions: ReadonlyArray<{
          position: number;
          publishedQuestionRevisionTuple: typeof studentViewQuestion;
        }>;
      }>;
    }> => {
      studentViewTransport.push("getInstructorStudentView");
      return Promise.resolve({
        assessmentEditNumber: "1",
        status: "released",
        title: "Protein structure practice",
        instructions: "",
        displayTimeZone: "America/Chicago",
        delivery: {
          available_at: null,
          due_at: null,
          closes_at: null,
          assessment_attempt_time_limit_seconds: null,
          attempt_limit: null,
          late_work_rule: "reject",
        },
        entries: [
          {
            kind: "presented",
            authoredPosition: 0,
            questions: [
              {
                position: 1,
                publishedQuestionRevisionTuple: studentViewQuestion,
              },
            ],
          },
        ],
      });
    },
    getInstructorStudentViewQuestion: (): Promise<{
      publishedQuestionRevisionTuple: typeof studentViewQuestion;
      prompt: ReadonlyArray<{ kind: "text"; markdown: string }>;
      response: { kind: "fillIn"; maxCharacters: number };
    }> => {
      studentViewTransport.push("getInstructorStudentViewQuestion");
      return Promise.resolve({
        publishedQuestionRevisionTuple: studentViewQuestion,
        prompt: [{ kind: "text", markdown: "Which atoms form a peptide bond?" }],
        response: { kind: "fillIn", maxCharacters: 40 },
      });
    },
    downloadCourseGradebook: (): Promise<Blob> =>
      Promise.resolve(new Blob(["roster_id\n"], { type: "text/csv" })),
    searchQuestionLibrary: (): Promise<QuestionSearchPage> =>
      Promise.resolve(fastUiQuestionLibraryPage()),
    resolveQuestion: (): Promise<unknown> => Promise.resolve(inspectionQuestionDetails().summary),
    getQuestionLineage: (): Promise<unknown> =>
      Promise.resolve({
        summary: inspectionQuestionDetails().summary,
        viewerMayArchive: true,
        questionAvailabilityEditNumber: "1",
      }),
    getQuestionStar: (): Promise<unknown> =>
      Promise.resolve({ starCount: 0, viewerHasStarred: false, starredInstructors: [] }),
    getQuestionWatch: (): Promise<unknown> => Promise.resolve({ watching: false }),
  };
  // The current-source App reaches the typed query subset above plus the
  // read-only Instructor Student View methods. The Course Instance surface
  // receives explicit identity and an empty Assessment list.
  return {
    api: { client, queries } as unknown as ApplicationApi<OrdinaryBrowserApiClient>,
    counts: {
      assessments: () => assessmentQueries,
      courseInstances: () => courseInstanceQueries,
      revokedRosterIds: (): readonly string[] => revokedRosterIds.slice(),
    },
    holdLiveCourseRoster: (): void => {
      holdRoster = true;
    },
    releaseLiveCourseRoster: (): void => {
      holdRoster = false;
      releaseRoster?.();
      releaseRoster = undefined;
    },
    failNextRosterRevoke: (): void => {
      failNextRevoke = true;
    },
    clearStudentViewTransport: (): void => {
      studentViewTransport.length = 0;
    },
    studentViewTransport: (): readonly string[] => studentViewTransport.slice(),
    seedAuthoringClassification,
    vocabularyWrites: (): readonly string[] => vocabularyWrites.slice(),
  };
}

function instructorSession(): AuthenticatedSession {
  return {
    authenticated: true,
    account: { id: "account-m10", userRole: "instructor" },
  };
}

function activeFixtureCourse(
  courseInstanceId: CourseInstanceView["courseInstance"]["id"],
): CourseInstanceSummary {
  const course = instructorCourseRouteData(courseInstanceId).summary;
  return {
    id: courseInstanceId,
    shortName: course.shortName,
    longName: course.longName,
    classification: FIXTURE_CLASSIFICATION,
    lifecycleState: "active",
    courseEditNumber: "1",
    theme: "grass",
    term: {
      startDate: "2026-01-12",
      endDate: "2026-05-08",
    },
  };
}

/** The controlled current-source course page is an instructor-owned Course Instance surface. */
function instructorCourseRouteData(courseInstanceId: string): CourseRouteView {
  const course = courseRouteData(courseInstanceId);
  return {
    ...course,
    summary: {
      ...course.summary,
      shortName: "BCHM 355",
      longName: "BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)",
      role: "instructor",
    },
  };
}

export interface RibbonShellHarness {
  readonly dispose: () => void;
  readonly disposeCurrent: () => void;
  readonly currentNavigate: (pathname: string) => void;
  readonly currentPathname: () => string;
  readonly fixtureNavigate: (pathname: string) => void;
  /** Navigates a fixture through a declared signed-in User Role and Route ID. */
  readonly fixtureNavigateRoute: (userRole: UserRole, routeId: RouteId) => void;
  readonly fixturePathname: () => string;
  readonly scopeRequestCount: (courseInstanceId: string) => number;
  readonly assessmentQueryCount: () => number;
  readonly courseInstanceQueryCount: () => number;
  readonly revokedRosterIds: () => readonly string[];
  readonly holdLiveCourseRoster: () => void;
  readonly releaseLiveCourseRoster: () => void;
  readonly failNextRosterRevoke: () => void;
  readonly clearStudentViewTransport: () => void;
  readonly studentViewTransport: () => readonly string[];
  readonly seedAuthoringClassification: () => void;
  readonly vocabularyWrites: () => readonly string[];
  readonly questionLibrarySeed: () => string;
  readonly questionLibrarySeedRows: () => ReadonlyArray<{ id: string; title: string }>;
  readonly releaseSession: () => void;
  readonly releaseCourseScope: (courseInstanceId: string) => void;
  readonly throwFixtureContent: (value: boolean) => void;
  readonly signOutActions: () => number;
}

function withSelectedControl<Id extends RibbonDestinationId>(
  controls: ReadonlyArray<RibbonControlModel<Id>>,
  selectedId: Id,
): ReadonlyArray<RibbonControlModel<Id>> {
  return controls.map((control) => ({ ...control, selected: control.id === selectedId }));
}

function withSelectedTaskControl(
  areas: ReadonlyArray<RibbonTaskAreaModel>,
  selectedId: string,
): ReadonlyArray<RibbonTaskAreaModel> {
  return areas.map((area) => ({
    ...area,
    controls: area.controls.map((control) => ({ ...control, selected: control.id === selectedId })),
  }));
}

function productFixture(selectedTab: "courses" | "questions" | "productAssessments"): RibbonModel {
  const source = M6_RIBBON_FIXTURES.productInstructor;
  const taskAreas: ReadonlyArray<RibbonTaskAreaModel> =
    selectedTab === "questions"
      ? withSelectedTaskControl(source.taskAreas, "searchQuestionLibrary")
      : [
          {
            id: selectedTab === "courses" ? "instructorCourses" : "instructorAssessments",
            controls: [],
          },
        ];
  return {
    ...source,
    tabs: withSelectedControl(source.tabs, selectedTab),
    taskAreas,
  };
}

/** Explicit presentation-only projection for the structural shell fixture. */
function fixtureModelForPathname(pathname: string): RibbonModel {
  const route = routeContractForPathname(pathname);
  if (route === undefined || route.id === "signIn") return productFixture("courses");
  const userRole = route.requiredUserRoles.includes("student") ? "student" : "instructor";
  return materializeRibbonRoute(userRole, route.id).model;
}

/** Structural-shell-only content. Fast route cases mount the production App instead. */
function FixtureContent(props: {
  readonly pathname: string;
  readonly shouldThrow: () => boolean;
}): JSX.Element {
  const { presentCourseTheme } = useAppearance();

  function presentOceanTheme(): void {
    presentCourseTheme("ocean");
  }

  if (props.shouldThrow()) throw new Error("Shell fixture content sentinel failure");
  return (
    <PageFrame routeSurface="ribbonShellFixture" title="Fixture page">
      <p>Fixture content</p>
      <button type="button" data-ribbon-theme-swap onClick={presentOceanTheme}>
        Present Ocean course theme
      </button>
    </PageFrame>
  );
}

const ribbonFormatFallback: ResponseFormatValidator = () =>
  Promise.reject(new Error("Ribbon evidence does not validate responses."));
const ribbonTimerFallback: TimerEvaluator = () =>
  Promise.reject(new Error("Ribbon evidence does not evaluate timers."));
const ribbonCapabilityFallback: CapabilityValidator = () =>
  Promise.reject(new Error("Ribbon evidence does not validate assessment config."));
const ribbonWasmFacade: WasmFacade = {
  mode: "serverFallback",
  validateResponseFormat: ribbonFormatFallback,
  questionAttemptTimingDecision: ribbonTimerFallback,
  assessmentAttemptRemainingMilliseconds: () =>
    Promise.reject(new Error("Ribbon evidence does not measure remaining time.")),
  validateAssessmentConfig: ribbonCapabilityFallback,
  previewPleDraft: (request) =>
    Promise.resolve({
      kind: "unavailable",
      backend: request.questionBackend,
      capability: "offlinePreview",
    }),
  verifyNativeStaticPresentationDescriptor: () => Promise.resolve({ kind: "unavailable" }),
};

/**
 * Mounts two deliberately labelled cases. The first is the current-source routed App;
 * the second uses only ApplicationShell's explicit model seam for structural
 * navigation/error evidence and never changes route admission.
 */
export function mountRibbonShellHarness(target: HTMLElement): RibbonShellHarness {
  const currentHistory = createMemoryHistory();
  const fixtureHistory = createMemoryHistory();
  const [fixtureShouldThrow, setFixtureShouldThrow] = createSignal(false);
  const [fixtureMaterializedRoute, setFixtureMaterializedRoute] = createSignal<
    ReturnType<typeof materializeRibbonRoute> | undefined
  >(undefined);
  const [signOutActions, setSignOutActions] = createSignal(0);
  let logoutAttempts = 0;
  const currentDeferredScopes = deferredCourseScopes();
  const currentSession = deferredSession();
  const currentPresentation = presentationApi(currentDeferredScopes);

  function fixtureNavigate(pathname: string): void {
    setFixtureMaterializedRoute(undefined);
    fixtureHistory.set({ value: pathname });
  }

  function fixtureNavigateRoute(userRole: UserRole, routeId: RouteId): void {
    const materializedRoute = materializeRibbonRoute(userRole, routeId);
    setFixtureMaterializedRoute(materializedRoute);
    fixtureHistory.set({ value: materializedRoute.pathname });
  }

  function FixtureInterior(): JSX.Element {
    const location = useLocation();
    function fixtureContent(pathname: string): JSX.Element {
      return <FixtureContent pathname={pathname} shouldThrow={fixtureShouldThrow} />;
    }
    function fixtureRibbonModel(): RibbonModel {
      return fixtureMaterializedRoute()?.model ?? fixtureModelForPathname(location.pathname);
    }
    return (
      <ApplicationShell
        pathname={() => location.pathname}
        ribbonModel={fixtureRibbonModel}
        content={fixtureContent}
      />
    );
  }

  const currentTarget = document.createElement("section");
  currentTarget.dataset.m10Case = "current-production";
  currentTarget.setAttribute("aria-label", "Current-source routed application");
  const fixtureTarget = document.createElement("section");
  fixtureTarget.dataset.m10Case = "fixture-shell";
  fixtureTarget.setAttribute("aria-label", "Structural fixture shell evidence");
  target.replaceChildren(currentTarget, fixtureTarget);

  const disposeCurrentRender = render(
    () => (
      <ApplicationApiProvider applicationApi={currentPresentation.api}>
        <SessionProvider
          getSession={currentSession.waitForSession}
          logout={() => Promise.resolve()}
          advanceSessionBoundary={() => undefined}
        >
          <InstalledWasmFacadeProvider facade={ribbonWasmFacade}>
            <MemoryRouter history={currentHistory} root={App}>
              {[...appRoutes, notFoundRoute]}
            </MemoryRouter>
          </InstalledWasmFacadeProvider>
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    currentTarget,
  );
  const disposeFixtureRender = render(
    () => (
      <ApplicationApiProvider applicationApi={presentationApi().api}>
        <SessionProvider
          getSession={() => Promise.resolve(instructorSession())}
          logout={() => {
            logoutAttempts += 1;
            setSignOutActions((count) => count + 1);
            if (logoutAttempts === 1) {
              return Promise.reject(new Error("fixture first sign-out remains unconfirmed"));
            }
            return Promise.resolve();
          }}
          advanceSessionBoundary={() => undefined}
        >
          <p class="sr-only">
            Structural fixture evidence, not a dist or real-stack browser workflow.
          </p>
          <MemoryRouter history={fixtureHistory} root={FixtureInterior} />
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    fixtureTarget,
  );
  let currentDisposed = false;
  function disposeCurrent(): void {
    if (currentDisposed) return;
    currentDisposed = true;
    disposeCurrentRender();
  }
  function dispose(): void {
    disposeCurrent();
    disposeFixtureRender();
  }

  return {
    dispose,
    disposeCurrent,
    currentNavigate: (pathname: string) => currentHistory.set({ value: pathname }),
    currentPathname: currentHistory.get,
    fixtureNavigate,
    fixtureNavigateRoute,
    fixturePathname: fixtureHistory.get,
    scopeRequestCount: currentDeferredScopes.requestCount,
    assessmentQueryCount: currentPresentation.counts.assessments,
    courseInstanceQueryCount: currentPresentation.counts.courseInstances,
    revokedRosterIds: currentPresentation.counts.revokedRosterIds,
    holdLiveCourseRoster: currentPresentation.holdLiveCourseRoster,
    releaseLiveCourseRoster: currentPresentation.releaseLiveCourseRoster,
    failNextRosterRevoke: currentPresentation.failNextRosterRevoke,
    clearStudentViewTransport: currentPresentation.clearStudentViewTransport,
    studentViewTransport: currentPresentation.studentViewTransport,
    seedAuthoringClassification: currentPresentation.seedAuthoringClassification,
    vocabularyWrites: currentPresentation.vocabularyWrites,
    questionLibrarySeed: () => FAST_UI_QUESTION_LIBRARY_SEED,
    questionLibrarySeedRows: () =>
      fastUiQuestionLibraryPage().items.map((item) => ({
        id: item.summary.questionId,
        title: item.summary.metadata.questionTitle,
      })),
    releaseSession: currentSession.release,
    releaseCourseScope: currentDeferredScopes.release,
    throwFixtureContent: setFixtureShouldThrow,
    signOutActions,
  };
}
