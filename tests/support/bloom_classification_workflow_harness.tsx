// Mounts the shipped Assessment Question editor and Question Library browse page.

import { Show, type JSX } from "solid-js";
import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { render } from "solid-js/web";

import type { AssessmentEntry } from "../../generated/api/AssessmentEntry";
import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import type {
  LiveAssessmentWorkspace,
  SaveLiveAssessmentInput,
} from "../../src/api/assessment_release";
import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type {
  ContentClassificationClient,
  ContentClassificationItem,
} from "../../src/api/content_classification";
import { LiveAssessmentWorkspaceConflictError } from "../../src/api/http_client/assessment_release";
import type { QuestionBulkMetadataClient } from "../../src/api/question_bulk_metadata";
import { SessionProvider, useSessionBootstrap } from "../../src/auth/session_context";
import { LibraryPage } from "../../src/pages/library_page";
import type { QuestionLibraryBrowseQuery } from "../../src/pages/library_page_model";
import { AssessmentWorkspaceLivePage } from "../../src/pages/assessment_workspace/assessment_workspace_live_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

const courseInstanceId = "CI7K3M2QAZ";
const assessmentId = "A8H4N6PA6";
const disciplineUuid = "11111111-1111-4111-8111-111111111111";
const rememberId = "00000000-0000-4000-8000-000000000001";
const poolId = "00000000-0000-4000-8000-000000000002";
const applyId = "00000000-0000-4000-8000-000000000003";
const evaluateId = "00000000-0000-4000-8000-000000000004";

export interface BloomAssessmentEntryFact {
  readonly id: string;
  readonly title: string;
  readonly cognitiveProcess: BloomClassificationView["cognitiveProcess"];
  readonly knowledgeDimension: BloomClassificationView["knowledgeDimension"];
}

export interface BloomAssessmentSave {
  readonly courseInstanceId: string;
  readonly assessmentId: string;
  readonly expectedAssessmentEditNumber: string;
  readonly entryIds: readonly string[];
}

export interface BloomLibrarySearch {
  readonly bloomCognitiveProcess: QuestionLibraryBrowseQuery["bloomCognitiveProcess"];
  readonly bloomKnowledgeDimension: QuestionLibraryBrowseQuery["bloomKnowledgeDimension"];
}

interface BloomWorkflowWindow {
  bloomAssessmentEntries: readonly BloomAssessmentEntryFact[];
  bloomAssessmentSaves: BloomAssessmentSave[];
  bloomLibrarySearches: BloomLibrarySearch[];
}

let disposeMount: (() => void) | undefined;

function workflowWindow(): BloomWorkflowWindow {
  return window as unknown as BloomWorkflowWindow;
}

function bloom(
  cognitiveProcess: BloomClassificationView["cognitiveProcess"],
  knowledgeDimension: BloomClassificationView["knowledgeDimension"],
): BloomClassificationView {
  return { cognitiveProcess, knowledgeDimension, classificationEditNumber: "1" };
}

function fixedEntry(
  id: string,
  publishedQuestionId: string,
): Extract<AssessmentEntry, { readonly kind: "fixedQuestion" }> {
  return {
    kind: "fixedQuestion",
    id,
    publishedQuestionRevisionTuple: { publishedQuestionId, revisionNumber: 1 },
    pointsPossible: "1",
    availability: "available",
    scoringRule: "normal",
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
  };
}

function assessmentEntries(): {
  readonly facts: readonly BloomAssessmentEntryFact[];
  readonly entries: readonly AssessmentEntry[];
  readonly questions: LiveAssessmentWorkspace["questions"];
} {
  const evaluate = fixedEntry(evaluateId, "ABCD-XEFG");
  const apply = fixedEntry(applyId, "AAAA-2BBB");
  const remember = fixedEntry(rememberId, "7K3M-79QP");
  const pool: Extract<AssessmentEntry, { readonly kind: "questionPool" }> = {
    kind: "questionPool",
    id: poolId,
    questionPoolId: "2R5X-E7YA",
    questionPoolEditNumber: 1,
    availability: "available",
    scoringRule: "normal",
    selectionCount: 1,
    pointsPerItem: "1",
    selectionRule: { selectedQuestionOrder: "randomOrder" },
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
  };
  const facts: readonly BloomAssessmentEntryFact[] = [
    {
      id: evaluateId,
      title: "Evaluate question",
      cognitiveProcess: "Evaluate",
      knowledgeDimension: "Procedural Knowledge",
    },
    {
      id: poolId,
      title: "Apply pool",
      cognitiveProcess: "Apply",
      knowledgeDimension: "Conceptual Knowledge",
    },
    {
      id: applyId,
      title: "Apply fixed question",
      cognitiveProcess: "Apply",
      knowledgeDimension: "Conceptual Knowledge",
    },
    {
      id: rememberId,
      title: "Remember question",
      cognitiveProcess: "Remember",
      knowledgeDimension: "Factual Knowledge",
    },
  ];
  return {
    facts,
    entries: [evaluate, pool, apply, remember],
    questions: [
      {
        publishedQuestionRevisionTuple: evaluate.publishedQuestionRevisionTuple,
        questionTitle: "Evaluate question",
        description: "Evaluate question",
        bloom: bloom("Evaluate", "Procedural Knowledge"),
      },
      {
        publishedQuestionRevisionTuple: apply.publishedQuestionRevisionTuple,
        questionTitle: "Apply fixed question",
        description: "Apply fixed question",
        bloom: bloom("Apply", "Conceptual Knowledge"),
      },
      {
        publishedQuestionRevisionTuple: remember.publishedQuestionRevisionTuple,
        questionTitle: "Remember question",
        description: "Remember question",
        bloom: bloom("Remember", "Factual Knowledge"),
      },
    ],
  };
}

function workspaceFrom(
  entries: readonly AssessmentEntry[],
  questions: LiveAssessmentWorkspace["questions"],
  assessmentEditNumber: string,
): LiveAssessmentWorkspace {
  return {
    id: assessmentId,
    assessmentEditNumber,
    status: "unreleased",
    origin: { kind: "direct" },
    assessmentType: "regular_assignment",
    title: "Bloom sort review",
    instructions: "",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assessmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {
      questionVariationRule: "newVariation",
      assessmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
      hints: "never",
      worked_solutions: "never",
    },
    displayTimeZone: "America/Chicago",
    entries: [...entries],
    questions: [...questions],
  };
}

function queries(): ApplicationApi<OrdinaryBrowserApiClient>["queries"] {
  return {
    courseScope(): Promise<never> {
      return Promise.reject(new Error("This Bloom workflow proof does not load a Course theme."));
    },
    assessmentAttemptScope(): Promise<never> {
      return Promise.reject(new Error("This Bloom workflow proof does not load an Attempt."));
    },
    assessmentAttemptHistory(): Promise<never> {
      return Promise.reject(new Error("This Bloom workflow proof does not load Attempt history."));
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>["queries"];
}

function biology(): ContentClassificationItem {
  return { uuid: disciplineUuid, name: "Biology", isRetired: false };
}

/** Opens the shipped Assessment Question editor with mixed fixed and Pool Entries. */
export function mountBloomAssessmentWorkflow(target: HTMLElement): void {
  disposeMount?.();
  const prepared = assessmentEntries();
  const saves: BloomAssessmentSave[] = [];
  const published = workflowWindow();
  published.bloomAssessmentEntries = prepared.facts;
  published.bloomAssessmentSaves = saves;
  let current = workspaceFrom(prepared.entries, prepared.questions, "1");
  let saveCount = 0;
  const applicationApi = {
    client: {
      getLiveAssessmentWorkspace: () => Promise.resolve({ workspace: current }),
      getAssessmentQuestionPoolFork: () =>
        Promise.resolve({
          metadata: {
            title: "Apply pool",
            description: "Apply pool",
            disciplineUuid,
            disciplineName: "Biology",
            disciplineIsRetired: false,
            subjectUuid: "",
            topicUuid: null,
            subtopicUuid: null,
            tags: [],
          },
          assessmentEntryId: poolId,
          questionPoolId: "2R5X-E7YA",
          questionPoolEditNumber: 1,
          selectionCount: 1,
          bloom: bloom("Apply", "Conceptual Knowledge"),
          members: [],
        }),
      saveLiveAssessment: (
        loadedCourseId: string,
        loadedAssessmentId: string,
        input: SaveLiveAssessmentInput,
        expectedAssessmentEditNumber: string,
      ) => {
        saves.push({
          courseInstanceId: loadedCourseId,
          assessmentId: loadedAssessmentId,
          expectedAssessmentEditNumber,
          entryIds: input.entries.map((entry) => entry.id),
        });
        saveCount += 1;
        if (saveCount > 1) {
          return Promise.reject(
            new LiveAssessmentWorkspaceConflictError(
              `/api/course-instances/${courseInstanceId}/assessments/${assessmentId}`,
            ),
          );
        }
        current = { ...workspaceFrom(input.entries, current.questions, "2"), title: input.title };
        return Promise.resolve({ workspace: current });
      },
      listDisciplinesIncludingRetired: () => Promise.resolve([biology()]),
      listSubjects: () => Promise.resolve([]),
      listTopics: () => Promise.resolve([]),
      listSubtopics: () => Promise.resolve([]),
    },
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  (window as unknown as { bloomApplicationApi: typeof applicationApi }).bloomApplicationApi =
    applicationApi;
  const pathname = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/questions`;
  const history = createMemoryHistory();
  history.set({ value: pathname, replace: true });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <MemoryRouter history={history}>
            <Route
              path="/instructor/courses/:courseInstanceId/assessments/:assessmentId/questions"
              component={() => <AssessmentWorkspaceLivePage section="questions" />}
            />
          </MemoryRouter>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}

function libraryPage(): {
  readonly items: readonly Record<string, unknown>[];
  readonly aggregates: readonly Record<string, unknown>[];
  readonly nextCursor: null;
  readonly facetTruncation: {
    readonly authorNames: false;
    readonly tags: false;
    readonly subjects: false;
    readonly topics: false;
  };
} {
  return {
    items: [
      {
        displayId: "7K3M-79QP",
        publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
        questionTitle: "Cell division",
        summary: "Answer-free summary",
        bloom: bloom("Remember", "Factual Knowledge"),
        disciplineName: "Biology",
        disciplineIsRetired: false,
        questionFormat: "pleQuestionJson",
        authorNames: [],
        capabilities: [],
        questionLicense: "CC-BY-4.0",
        evidence: { state: "unavailable" },
      },
    ],
    aggregates: [
      { facet: "bloomCognitiveProcess", value: "Remember", count: 1 },
      { facet: "bloomKnowledgeDimension", value: "Factual Knowledge", count: 1 },
    ],
    nextCursor: null,
    facetTruncation: { authorNames: false, tags: false, subjects: false, topics: false },
  };
}

function classificationClient(): ContentClassificationClient {
  return {
    listDisciplines: () => Promise.resolve([biology()]),
    listDisciplinesIncludingRetired: () => Promise.resolve([biology()]),
    listSubjects: () => Promise.resolve([]),
    listTopics: () => Promise.resolve([]),
    listSubtopics: () => Promise.resolve([]),
    requestContentDiscipline: () =>
      Promise.reject(new Error("Discipline requests are not under test.")),
    createSubject: () => Promise.reject(new Error("Subject creation is not under test.")),
    acceptSubjectDiscipline: () =>
      Promise.reject(new Error("Subject acceptance is not under test.")),
    createTopic: () => Promise.reject(new Error("Topic creation is not under test.")),
    createSubtopic: () => Promise.reject(new Error("Subtopic creation is not under test.")),
  };
}

function metadataClient(): QuestionBulkMetadataClient {
  return {
    getCurrentQuestionBulkMetadata: () =>
      Promise.reject(new Error("Metadata reading is not under test.")),
    updateQuestionBulkMetadata: () =>
      Promise.reject(new Error("Metadata editing is not under test.")),
  };
}

function LibraryHost(): JSX.Element {
  return (
    <LibraryPage
      mode="browse"
      repository={{
        search: (query) => {
          workflowWindow().bloomLibrarySearches.push({
            bloomCognitiveProcess: query.bloomCognitiveProcess,
            bloomKnowledgeDimension: query.bloomKnowledgeDimension,
          });
          return Promise.resolve(libraryPage());
        },
      }}
      metadataClient={metadataClient()}
      classificationClient={classificationClient()}
      questionPoolClient={{
        createQuestionPool: () => Promise.reject(new Error("Pool creation is not under test.")),
      }}
      getQuestionDetails={() => Promise.reject(new Error("Question details are not under test."))}
    />
  );
}

function AuthenticatedLibrary(): JSX.Element {
  const bootstrap = useSessionBootstrap();
  return (
    <Show
      when={bootstrap.state().kind === "authenticated"}
      fallback={<p role="status">Loading Question Library session</p>}
    >
      <LibraryHost />
    </Show>
  );
}

/** Opens shipped Question Library browse so a Bloom filter reaches repository search. */
export function mountBloomLibraryBrowse(target: HTMLElement): void {
  disposeMount?.();
  workflowWindow().bloomLibrarySearches = [];
  const applicationApi = {
    client: {},
    queries: queries(),
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const pathname = "/library/browse";
  const history = createMemoryHistory();
  history.set({ value: pathname, replace: true });
  disposeMount = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={pathname}>
          <SessionProvider
            getSession={() =>
              Promise.resolve({
                authenticated: true,
                account: { id: "U0000035E", userRole: "instructor" },
              })
            }
            logout={() => Promise.resolve()}
            advanceSessionBoundary={() => undefined}
          >
            <MemoryRouter history={history}>
              <Route path="/library/browse" component={AuthenticatedLibrary} />
            </MemoryRouter>
          </SessionProvider>
        </RouteScopeProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
}
