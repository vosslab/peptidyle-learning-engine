// assignment_editor_live_page.tsx - session and course-role gate for assignment editing.

import { useParams } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import type { AuthSession, CourseSummary } from "../api/contracts";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { useCourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import { AssignmentEditorPage, type AssignmentEditorMode } from "./assignment_editor_page";
import { createAssignmentEditorRepository } from "./assignment_editor_repository";
import { resolveAssignmentRoute } from "../navigation/resolved_route";

type CourseGate =
  | { readonly kind: "loading" }
  | {
      readonly kind: "allowed";
      readonly course: CourseSummary;
      readonly courseId: CourseId;
      readonly mode: AssignmentEditorMode;
    }
  | { readonly kind: "denied" }
  | { readonly kind: "unavailable" };

function AssignmentEditingDenied(): JSX.Element {
  return (
    <section
      class="page"
      data-route-surface="assignmentEditor"
      data-editor-state="denied"
      aria-live="polite"
    >
      <p class="eyebrow">Instructor course design</p>
      <h1>You do not manage this course</h1>
      <p>Return to your courses and choose an assignment in a course you manage.</p>
    </section>
  );
}

function AssignmentEditingUnavailable(): JSX.Element {
  return (
    <section
      class="page route-error"
      data-route-surface="assignmentEditor"
      data-editor-state="unavailable"
      role="alert"
    >
      <p class="eyebrow">Instructor course design</p>
      <h1>This assignment editor is unavailable</h1>
      <p>Return to your courses and choose an assignment you manage.</p>
    </section>
  );
}

interface AuthenticatedAssignmentEditorProps {
  readonly session: AuthSession;
}

function AuthenticatedAssignmentEditor(props: AuthenticatedAssignmentEditorProps): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const scopedRoute = useCourseThemeRouteData();
  const [gate, setGate] = createSignal<CourseGate>({ kind: "loading" });
  const courseId = (): CourseId | undefined =>
    scopedRoute?.kind === "course" ? scopedRoute.course.summary.id : undefined;
  const allowedGate = (): Extract<CourseGate, { readonly kind: "allowed" }> | undefined => {
    const current = gate();
    return current.kind === "allowed" ? current : undefined;
  };

  async function checkCourseAccess(): Promise<void> {
    const selectedCourseId = courseId();
    if (selectedCourseId === undefined) {
      setGate({ kind: "unavailable" });
      return;
    }
    try {
      const course =
        scopedRoute?.kind === "course"
          ? scopedRoute.course.summary
          : await runtime.client.getCourse(selectedCourseId);
      if (course.id !== selectedCourseId || course.tenant !== props.session.tenant) {
        setGate({ kind: "unavailable" });
        return;
      }
      if (course.role !== "instructor") {
        setGate({ kind: "denied" });
        return;
      }
      let mode: AssignmentEditorMode = { kind: "create" };
      const assignmentReference = params["assignmentRef"];
      if (assignmentReference !== undefined) {
        const assignment = await resolveAssignmentRoute(runtime.client, assignmentReference);
        if (assignment.courseId !== selectedCourseId) {
          setGate({ kind: "unavailable" });
          return;
        }
        mode = { kind: "edit", assignmentId: assignment.assignmentId };
      }
      setGate({ kind: "allowed", course, courseId: selectedCourseId, mode });
    } catch {
      setGate({ kind: "unavailable" });
    }
  }

  onMount(() => void checkCourseAccess());

  return (
    <Show
      when={allowedGate()}
      keyed
      fallback={
        <Show
          when={gate().kind === "loading"}
          fallback={
            gate().kind === "denied" ? (
              <AssignmentEditingDenied />
            ) : (
              <AssignmentEditingUnavailable />
            )
          }
        >
          <section class="page" data-route-surface="assignmentEditorGate" aria-busy="true">
            <p class="loading-state" role="status">
              Checking course editing access...
            </p>
          </section>
        </Show>
      }
    >
      {(allowed) => (
        <AssignmentEditorPage
          repository={createAssignmentEditorRepository(runtime.client)}
          courseId={allowed.courseId}
          courseReference={allowed.course.reference}
          mode={allowed.mode}
          tenant={props.session.tenant}
        />
      )}
    </Show>
  );
}

/** Relies on the outer route role boundary, then retains course-local ownership checks. */
export function AssignmentEditorLivePage(): JSX.Element {
  const session = useSessionBootstrap();
  const authenticatedSession = (): AuthSession | undefined => {
    const state = session.state();
    return state.kind === "authenticated" ? state.session : undefined;
  };
  return (
    <Show when={authenticatedSession()} keyed fallback={<AssignmentEditingUnavailable />}>
      {(authenticated) => <AuthenticatedAssignmentEditor session={authenticated} />}
    </Show>
  );
}
