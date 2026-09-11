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

import type {
  ReleasedLiveAssignment,
  RevisionedLiveAssignmentWorkspace,
  SaveLiveAssignmentInput,
} from "../../api/assignment_release";
import { useApplicationApi } from "../../api/application_api";
import {
  parseAssignmentReference,
  parseCourseInstanceReference,
  type AssignmentRouteReference,
  type CourseInstanceRouteReference,
} from "../../navigation/public_route";
import "./assignment_workspace_authoring.css";
import { type AssignmentWorkspaceSection } from "./assignment_workspace_paths";
import { AssignmentWorkspaceOverviewPage } from "./assignment_workspace_overview_page";
import { AssignmentWorkspaceOperationsPage } from "./assignment_workspace_operations_page";
import { AssignmentWorkspacePoliciesPage } from "./assignment_workspace_policies_page";
import { AssignmentWorkspaceQuestionsPage } from "./assignment_workspace_questions_page";
import { AssignmentWorkspaceStudentViewPage } from "./assignment_workspace_student_view_page";
import "./assignment_workspace.css";

export interface AssignmentWorkspaceContextValue {
  readonly courseReference: CourseInstanceRouteReference;
  /** Shared direct resource and exact ETag for every child page. */
  readonly assignment: Accessor<RevisionedLiveAssignmentWorkspace>;
  readonly assignmentReference: AssignmentRouteReference;
  readonly save: (input: SaveLiveAssignmentInput) => Promise<RevisionedLiveAssignmentWorkspace>;
  readonly release: (etag: string) => Promise<ReleasedLiveAssignment>;
  readonly reloadAssignment: () => Promise<RevisionedLiveAssignmentWorkspace>;
}

const AssignmentWorkspaceContext = createContext<AssignmentWorkspaceContextValue>();

export function useAssignmentWorkspace(): AssignmentWorkspaceContextValue {
  const value = useContext(AssignmentWorkspaceContext);
  if (value === undefined) throw new Error("AssignmentWorkspaceLivePage is missing");
  return value;
}

type LoadState = "loading" | "unavailable" | "error";

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
function AssignmentWorkspaceLiveContent(props: AssignmentWorkspaceLivePageProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const [state, setState] = createSignal<LoadState>("loading");
  const [workspace, setWorkspace] = createSignal<AssignmentWorkspaceContextValue>();
  let retryButton: HTMLButtonElement | undefined;

  function registerRetryButton(element: HTMLButtonElement): void {
    retryButton = element;
  }

  async function load(): Promise<void> {
    setState("loading");
    const courseReference = parseCourseInstanceReference(params["courseRef"] ?? "");
    const assignmentReference = parseAssignmentReference(params["assignmentRef"] ?? "");
    if (courseReference === null || assignmentReference === null) {
      setState("unavailable");
      return;
    }
    try {
      const assignment = await applicationApi.client.getLiveAssignmentWorkspace(
        courseReference,
        assignmentReference,
      );
      const [currentAssignment, setCurrentAssignment] = createSignal(assignment);
      const reloadAssignment = async (): Promise<RevisionedLiveAssignmentWorkspace> => {
        const latest = await applicationApi.client.getLiveAssignmentWorkspace(
          courseReference,
          assignmentReference,
        );
        setCurrentAssignment(latest);
        return latest;
      };
      const save = async (
        input: SaveLiveAssignmentInput,
      ): Promise<RevisionedLiveAssignmentWorkspace> => {
        const saved = await applicationApi.client.saveLiveAssignment(
          courseReference,
          assignmentReference,
          input,
          currentAssignment().etag,
        );
        setCurrentAssignment(saved);
        return saved;
      };
      const release = async (etag: string): Promise<ReleasedLiveAssignment> =>
        await applicationApi.client.releaseLiveAssignment(
          courseReference,
          assignmentReference,
          etag,
        );
      setWorkspace({
        courseReference,
        assignment: currentAssignment,
        assignmentReference,
        release,
        save,
        reloadAssignment,
      });
    } catch (error: unknown) {
      const failureState: LoadState = error instanceof Error ? "error" : "unavailable";
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
            <WorkspaceChild section={props.section} />
          </section>
        </AssignmentWorkspaceContext.Provider>
      )}
    </Show>
  );
}

/** Mounts the direct-resource loader; authorization remains server-owned. */
export function AssignmentWorkspaceLivePage(props: AssignmentWorkspaceLivePageProps): JSX.Element {
  return <AssignmentWorkspaceLiveContent {...props} />;
}
