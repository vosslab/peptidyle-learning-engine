// assignment_workspace_live_page.tsx - one exact-authority loader for workspace child pages.

import { A, useParams } from "@solidjs/router";
import {
  createContext,
  onMount,
  Show,
  useContext,
  createSignal,
  type Accessor,
  type JSX,
} from "solid-js";

import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { CourseId } from "../../../generated/api/CourseId";
import type { ApiClient } from "../../api/client";
import type { AssignmentEditorDetail, CourseSummary } from "../../api/contracts";
import { useApplicationApi } from "../../api/application_api";
import { useSessionBootstrap } from "../../auth/session_context";
import {
  courseRouteView,
  useCourseThemeRouteData,
} from "../../features/course_appearance/course_theme_context";
import { resolveAssignmentRoute } from "../../navigation/resolved_route";
import {
  courseInstanceRouteReference,
  parseAssignmentReference,
  parseCourseInstanceReference,
  type AssignmentRouteReference,
  type CourseInstanceRouteReference,
} from "../../navigation/public_route";
import { createAssignmentEditorRepository } from "../assignment_editor_repository";
import "./assignment_workspace_authoring.css";
import {
  AssignmentWorkspaceNav,
  type AssignmentWorkspaceSection,
} from "./assignment_workspace_nav";
import { AssignmentWorkspaceOverviewPage } from "./assignment_workspace_overview_page";
import { AssignmentWorkspaceOperationsPage } from "./assignment_workspace_operations_page";
import { AssignmentWorkspacePoliciesPage } from "./assignment_workspace_policies_page";
import { AssignmentWorkspaceQuestionsPage } from "./assignment_workspace_questions_page";
import { AssignmentWorkspaceStudentViewPage } from "./assignment_workspace_student_view_page";
import {
  assignmentWorkspaceLoadFailureState,
  type AssignmentWorkspaceLoadState,
} from "./assignment_workspace_load_model";
import "./assignment_workspace.css";

export interface AssignmentWorkspaceContextValue {
  readonly course: CourseSummary;
  readonly courseId: CourseId;
  readonly courseReference: CourseInstanceRouteReference;
  /** Shared live detail; focused saves replace this value for every child page. */
  readonly assignment: Accessor<AssignmentEditorDetail>;
  readonly assignmentId: AssignmentId;
  readonly assignmentReference: AssignmentRouteReference;
  readonly client: ApiClient;
  readonly repository: ReturnType<typeof createAssignmentEditorRepository>;
  readonly replaceAssignment: (assignment: AssignmentEditorDetail) => void;
  readonly reloadAssignment: () => Promise<AssignmentEditorDetail>;
}

const AssignmentWorkspaceContext = createContext<AssignmentWorkspaceContextValue>();

export function useAssignmentWorkspace(): AssignmentWorkspaceContextValue {
  const value = useContext(AssignmentWorkspaceContext);
  if (value === undefined) throw new Error("AssignmentWorkspaceLivePage is missing");
  return value;
}

type LoadState = "loading" | AssignmentWorkspaceLoadState;

function WorkspaceState(props: {
  readonly state: LoadState;
  readonly retry: () => void;
  readonly registerRetryButton: (element: HTMLButtonElement) => void;
}): JSX.Element {
  if (props.state === "loading") {
    return (
      <section class="page assignment-workspace-state" data-route-surface="assignmentWorkspaceGate">
        <p class="eyebrow">Instructor assignment workspace</p>
        <p class="loading-state" role="status">
          Loading assignment workspace...
        </p>
      </section>
    );
  }
  if (props.state === "denied") {
    return (
      <section
        class="page assignment-workspace-state route-error"
        data-route-surface="assignmentWorkspaceGate"
        role="alert"
      >
        <p class="eyebrow">Instructor assignment workspace</p>
        <h1>You do not manage this course</h1>
        <p>Return to your courses and choose an assignment in a course you manage.</p>
        <A class="primary-link" href="/">
          Return to courses
        </A>
      </section>
    );
  }
  if (props.state === "error") {
    return (
      <section
        class="page assignment-workspace-state route-error"
        data-route-surface="assignmentWorkspaceGate"
        role="alert"
        aria-labelledby="assignment-workspace-load-error"
      >
        <p class="eyebrow">Instructor assignment workspace</p>
        <h1 id="assignment-workspace-load-error">Assignment workspace could not load</h1>
        <p>Try loading the current assignment again.</p>
        <button
          class="primary-action"
          type="button"
          onClick={props.retry}
          ref={props.registerRetryButton}
        >
          Retry loading assignment
        </button>
      </section>
    );
  }
  return (
    <section
      class="page assignment-workspace-state route-error"
      data-route-surface="assignmentWorkspaceGate"
      role="alert"
    >
      <p class="eyebrow">Instructor assignment workspace</p>
      <h1>This assignment workspace is unavailable</h1>
      <p>The selected assignment could not be found in this course.</p>
      <A class="primary-link" href="/">
        Return to courses
      </A>
    </section>
  );
}

function WorkspaceChild(props: { readonly section: AssignmentWorkspaceSection }): JSX.Element {
  switch (props.section) {
    case "overview":
      return <AssignmentWorkspaceOverviewPage />;
    case "questions":
      return <AssignmentWorkspaceQuestionsPage />;
    case "policies":
      return <AssignmentWorkspacePoliciesPage />;
    case "studentView":
      return <AssignmentWorkspaceStudentViewPage />;
    case "gradingOperations":
      return <AssignmentWorkspaceOperationsPage />;
  }
}

export interface AssignmentWorkspaceLivePageProps {
  readonly section: AssignmentWorkspaceSection;
}

/** Resolves public references, proves the exact course relationship, then loads one workspace detail. */
export function AssignmentWorkspaceLivePage(props: AssignmentWorkspaceLivePageProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const scopedRoute = useCourseThemeRouteData();
  const session = useSessionBootstrap();
  const [state, setState] = createSignal<LoadState>("loading");
  const [workspace, setWorkspace] = createSignal<AssignmentWorkspaceContextValue>();
  let retryButton: HTMLButtonElement | undefined;

  function registerRetryButton(element: HTMLButtonElement): void {
    retryButton = element;
  }

  async function load(): Promise<void> {
    setState("loading");
    const course =
      scopedRoute?.kind === "course" ? courseRouteView(scopedRoute).summary : undefined;
    const courseReference = parseCourseInstanceReference(params["courseRef"] ?? "");
    const assignmentReference = parseAssignmentReference(params["assignmentRef"] ?? "");
    if (course === undefined || courseReference === null || assignmentReference === null) {
      setState("unavailable");
      return;
    }
    const currentSession = session.state();
    if (
      currentSession.kind !== "authenticated" ||
      currentSession.session.account.role !== "instructor"
    ) {
      setState("denied");
      return;
    }
    if (
      course.role !== "instructor" ||
      courseInstanceRouteReference(course.reference) !== courseReference
    ) {
      setState("denied");
      return;
    }
    try {
      const identity = await resolveAssignmentRoute(applicationApi.client, assignmentReference);
      if (identity.courseId !== course.id) {
        setState("unavailable");
        return;
      }
      const assignment = await applicationApi.client.getAssignmentWorkspace(
        course.id,
        identity.assignmentId,
      );
      if (assignment.id !== identity.assignmentId || assignment.courseId !== course.id) {
        setState("unavailable");
        return;
      }
      const [currentAssignment, setCurrentAssignment] = createSignal(assignment);
      const replaceAssignment = (next: AssignmentEditorDetail): void => {
        if (next.id !== identity.assignmentId || next.courseId !== course.id) {
          throw new Error("Assignment update does not match the workspace authority");
        }
        setCurrentAssignment(next);
      };
      const reloadAssignment = async (): Promise<AssignmentEditorDetail> => {
        const latest = await applicationApi.client.getAssignmentWorkspace(
          course.id,
          identity.assignmentId,
        );
        replaceAssignment(latest);
        return latest;
      };
      setWorkspace({
        course,
        courseId: course.id,
        courseReference,
        assignment: currentAssignment,
        assignmentId: identity.assignmentId,
        assignmentReference,
        client: applicationApi.client,
        repository: createAssignmentEditorRepository(applicationApi.client),
        replaceAssignment,
        reloadAssignment,
      });
    } catch (error: unknown) {
      const failureState = assignmentWorkspaceLoadFailureState(error);
      setState(failureState);
      if (failureState === "error") {
        requestAnimationFrame(() => retryButton?.focus());
      }
    }
  }

  onMount(() => void load());

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
        <AssignmentWorkspaceContext.Provider value={loaded}>
          <section class="page assignment-workspace" data-route-surface="assignmentWorkspace">
            <AssignmentWorkspaceNav
              courseReference={loaded.courseReference}
              assignmentReference={loaded.assignmentReference}
              active={props.section}
            />
            <WorkspaceChild section={props.section} />
          </section>
        </AssignmentWorkspaceContext.Provider>
      )}
    </Show>
  );
}
