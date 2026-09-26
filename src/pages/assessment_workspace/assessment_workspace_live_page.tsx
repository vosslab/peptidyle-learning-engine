// assessment_workspace_live_page.tsx - one exact-authority loader for workspace child pages.

import { A, useLocation, useParams } from "@solidjs/router";
import {
  createContext,
  onCleanup,
  onMount,
  Show,
  useContext,
  createSignal,
  type Accessor,
  type JSX,
} from "solid-js";

import type {
  LiveAssessmentWorkspaceResponse,
  SaveBaseAssessmentPolicyInput,
  SaveLiveAssessmentInput,
  UnreleasedLiveAssessment,
} from "../../api/assessment_release";
import { useApplicationApi } from "../../api/application_api";
import { PageFrame } from "../../components/page_frame";
import {
  parseAssessmentId,
  parseCourseInstanceId,
  type AssessmentRouteId,
  type CourseInstanceRouteId,
} from "../../navigation/public_route";
import "./assessment_workspace_authoring.css";
import { type AssessmentWorkspaceSection } from "./assessment_workspace_paths";
import { AssessmentWorkspaceOverviewPage } from "./assessment_workspace_overview_page";
import { AssessmentWorkspacePoliciesPage } from "./assessment_workspace_policies_page";
import { AssessmentWorkspaceQuestionsPage } from "./assessment_workspace_questions_page";
import { AssessmentWorkspaceStudentViewPage } from "./assessment_workspace_student_view_page";
import "./assessment_workspace.css";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../../ribbon/route_scope_context";

export interface AssessmentWorkspaceContextValue {
  readonly courseInstanceId: CourseInstanceRouteId;
  /** Shared direct resource and exact Assessment Edit Number for every child page. */
  readonly assessment: Accessor<LiveAssessmentWorkspaceResponse>;
  readonly assessmentId: AssessmentRouteId;
  readonly save: (input: SaveLiveAssessmentInput) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly saveBaseAssessmentPolicy: (
    input: SaveBaseAssessmentPolicyInput,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly release: (
    expectedAssessmentEditNumber: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly unrelease: (confirmationTitle: string) => Promise<UnreleasedLiveAssessment>;
  readonly reloadAssessment: () => Promise<LiveAssessmentWorkspaceResponse>;
}

const AssessmentWorkspaceContext = createContext<AssessmentWorkspaceContextValue>();

export function useAssessmentWorkspace(): AssessmentWorkspaceContextValue {
  const value = useContext(AssessmentWorkspaceContext);
  if (value === undefined) throw new Error("AssessmentWorkspaceLivePage is missing");
  return value;
}

function assessmentStatusLabel(
  status: LiveAssessmentWorkspaceResponse["workspace"]["status"],
): string {
  switch (status) {
    case "unreleased":
      return "Unreleased";
    case "released":
      return "Released";
    case "closed":
      return "Closed";
    case "archived":
      return "Archived";
  }
}

/** Compact, current identity shared by the Question and Properties work areas. */
export function AssessmentWorkspaceIdentity(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const assessment = (): LiveAssessmentWorkspaceResponse["workspace"] =>
    workspace.assessment().workspace;
  return (
    <dl class="assessment-workspace-identity" aria-label="Current Assessment">
      <div>
        <dt>Assessment</dt>
        <dd>{assessment().title}</dd>
      </div>
      <div>
        <dt>Release status</dt>
        <dd data-assessment-release-status={assessment().status}>
          {assessmentStatusLabel(assessment().status)}
        </dd>
      </div>
      <div>
        <dt>Current edit</dt>
        <dd>{assessment().assessmentEditNumber}</dd>
      </div>
    </dl>
  );
}

type LoadState = "loading" | "unavailable" | "error";

function WorkspaceState(props: {
  readonly state: LoadState;
  readonly retry: () => void;
  readonly registerRetryButton: (element: HTMLButtonElement) => void;
}): JSX.Element {
  if (props.state === "loading") {
    return (
      <PageFrame
        routeSurface="assessmentWorkspaceGate"
        eyebrow="Instructor assessment workspace"
        title="Loading assessment workspace"
      >
        <p class="loading-state" role="status">
          Loading assessment workspace...
        </p>
      </PageFrame>
    );
  }
  if (props.state === "error") {
    return (
      <div role="alert">
        <PageFrame
          // Failure card on the content region. PageFrame still owns the stack.
          contentClass="route-error"
          routeSurface="assessmentWorkspaceGate"
          eyebrow="Instructor assessment workspace"
          title="Assessment workspace could not load"
          headingId="assessment-workspace-load-error"
        >
          <p>Try loading the current assessment again.</p>
          <button
            class="primary-action"
            type="button"
            onClick={props.retry}
            ref={props.registerRetryButton}
          >
            Retry loading assessment
          </button>
        </PageFrame>
      </div>
    );
  }
  return (
    <div role="alert">
      <PageFrame
        // Failure card on the content region. PageFrame still owns the stack.
        contentClass="route-error"
        routeSurface="assessmentWorkspaceGate"
        eyebrow="Instructor assessment workspace"
        title="This assessment workspace is unavailable"
      >
        <p>The selected assessment could not be found in this course.</p>
        <A class="primary-link" href="/">
          Return to courses
        </A>
      </PageFrame>
    </div>
  );
}

function WorkspaceChild(props: { readonly section: AssessmentWorkspaceSection }): JSX.Element {
  switch (props.section) {
    case "overview":
      return <AssessmentWorkspaceOverviewPage />;
    case "questions":
      return <AssessmentWorkspaceQuestionsPage />;
    case "policies":
      return <AssessmentWorkspacePoliciesPage />;
    case "studentView":
      return <AssessmentWorkspaceStudentViewPage />;
  }
}

export interface AssessmentWorkspaceLivePageProps {
  readonly section: AssessmentWorkspaceSection;
}

/** Resolves public IDs, proves the exact course relationship, then loads one workspace detail. */
function AssessmentWorkspaceLiveContent(props: AssessmentWorkspaceLivePageProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const location = useLocation();
  const params = useParams();
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  const [state, setState] = createSignal<LoadState>("loading");
  const [workspace, setWorkspace] = createSignal<AssessmentWorkspaceContextValue>();
  let activePublication: ReturnType<typeof routeScopePublication> | undefined;
  let retryButton: HTMLButtonElement | undefined;

  function registerRetryButton(element: HTMLButtonElement): void {
    retryButton = element;
  }

  async function load(): Promise<void> {
    const pathname = location.pathname;
    const publication = routeScopePublication();
    activePublication = publication;
    clearRouteScopeLabels(publication);
    setState("loading");
    const courseInstanceId = parseCourseInstanceId(params["courseInstanceId"] ?? "");
    const assessmentId = parseAssessmentId(params["assessmentId"] ?? "");
    if (courseInstanceId === null || assessmentId === null) {
      setState("unavailable");
      return;
    }
    try {
      const assessment = await applicationApi.client.getLiveAssessmentWorkspace(
        courseInstanceId,
        assessmentId,
      );
      if (location.pathname !== pathname) return;
      const [currentAssessment, setCurrentAssessment] = createSignal(assessment);
      const replaceCurrentAssessment = (next: LiveAssessmentWorkspaceResponse): void => {
        setCurrentAssessment(next);
        publishRouteScopeLabels(publication, { assessmentTitle: next.workspace.title });
      };
      const reloadAssessment = async (): Promise<LiveAssessmentWorkspaceResponse> => {
        const latest = await applicationApi.client.getLiveAssessmentWorkspace(
          courseInstanceId,
          assessmentId,
        );
        replaceCurrentAssessment(latest);
        return latest;
      };
      const save = async (
        input: SaveLiveAssessmentInput,
      ): Promise<LiveAssessmentWorkspaceResponse> => {
        const saved = await applicationApi.client.saveLiveAssessment(
          courseInstanceId,
          assessmentId,
          input,
          currentAssessment().workspace.assessmentEditNumber,
        );
        replaceCurrentAssessment(saved);
        return saved;
      };
      const saveBaseAssessmentPolicy = async (
        input: SaveBaseAssessmentPolicyInput,
      ): Promise<LiveAssessmentWorkspaceResponse> => {
        const saved = await applicationApi.client.saveBaseAssessmentPolicy(
          courseInstanceId,
          assessmentId,
          input,
          currentAssessment().workspace.assessmentEditNumber,
        );
        replaceCurrentAssessment(saved);
        return saved;
      };
      const release = async (
        expectedAssessmentEditNumber: string,
      ): Promise<LiveAssessmentWorkspaceResponse> => {
        const released = await applicationApi.client.releaseLiveAssessment(
          courseInstanceId,
          assessmentId,
          expectedAssessmentEditNumber,
        );
        replaceCurrentAssessment(released);
        return released;
      };
      const unrelease = async (confirmationTitle: string): Promise<UnreleasedLiveAssessment> => {
        const result = await applicationApi.client.unreleaseLiveAssessment(
          courseInstanceId,
          assessmentId,
          confirmationTitle,
          currentAssessment().workspace.assessmentEditNumber,
        );
        replaceCurrentAssessment({ workspace: result.assessment });
        return result;
      };
      setWorkspace({
        courseInstanceId,
        assessment: currentAssessment,
        assessmentId,
        release,
        unrelease,
        save,
        saveBaseAssessmentPolicy,
        reloadAssessment,
      });
      publishRouteScopeLabels(publication, { assessmentTitle: assessment.workspace.title });
    } catch (error: unknown) {
      clearRouteScopeLabels(publication);
      const failureState: LoadState = error instanceof Error ? "error" : "unavailable";
      setState(failureState);
      if (failureState === "error") {
        requestAnimationFrame(() => retryButton?.focus());
      }
    }
  }

  onMount(() => void load());
  onCleanup(() => {
    if (activePublication !== undefined) clearRouteScopeLabels(activePublication);
  });

  return (
    <Show
      when={workspace()}
      keyed
      fallback={
        <WorkspaceState
          state={state()}
          retry={() => void load()}
          registerRetryButton={registerRetryButton}
        />
      }
    >
      {(loaded) => (
        <AssessmentWorkspaceContext.Provider value={loaded}>
          <div class="assessment-workspace" data-route-surface="assessmentWorkspace">
            <WorkspaceChild section={props.section} />
          </div>
        </AssessmentWorkspaceContext.Provider>
      )}
    </Show>
  );
}

/** Mounts the direct-resource loader; authorization remains server-owned. */
export function AssessmentWorkspaceLivePage(props: AssessmentWorkspaceLivePageProps): JSX.Element {
  return <AssessmentWorkspaceLiveContent {...props} />;
}
