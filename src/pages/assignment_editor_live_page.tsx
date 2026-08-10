// assignment_editor_live_page.tsx - session and course-role gate for assignment editing.

import { useParams } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseSummary } from "../api/contracts";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap, type SessionBootstrapState } from "../auth/session_context";
import { useCourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import { AssignmentEditorPage } from "./assignment_editor_page";
import { createAssignmentEditorRepository } from "./assignment_editor_repository";

type CourseGate =
  | { readonly kind: "loading" }
  | { readonly kind: "allowed"; readonly course: CourseSummary }
  | { readonly kind: "denied" }
  | { readonly kind: "unavailable" };

function mayOpenInstructorSurface(session: SessionBootstrapState): boolean {
  if (session.kind !== "authenticated") return false;
  return session.session.user.roles.some((role) =>
    ["instructor", "publisher", "administrator"].includes(role),
  );
}

function AssignmentEditingDenied(): JSX.Element {
  return (
    <section
      class="page"
      data-route-surface="assignmentEditor"
      data-editor-state="denied"
      aria-live="polite"
    >
      <p class="eyebrow">Instructor course design</p>
      <h1>Assignment editing is not available for this account</h1>
      <p>Your learning space remains available. Ask a course administrator for editing access.</p>
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

/**
 * Route gate order is security-relevant: an account without an instructor role makes no
 * course, assignment, or catalog request; a course learner makes only the safe course check.
 */
export function AssignmentEditorLivePage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const scopedRoute = useCourseThemeRouteData();
  const session = useSessionBootstrap().state();
  const authenticated = session.kind === "authenticated" ? session.session : undefined;
  if (authenticated === undefined) return <AssignmentEditingUnavailable />;
  const sessionTenant = authenticated.tenant;
  const [gate, setGate] = createSignal<CourseGate>({ kind: "loading" });
  const courseId: CourseId | undefined = params["courseId"];
  const assignmentId: AssignmentId | undefined = params["assignmentId"];
  const allowedGlobalRole = mayOpenInstructorSurface(session);

  async function checkCourseAccess(): Promise<void> {
    if (!allowedGlobalRole) return;
    if (courseId === undefined || assignmentId === undefined) {
      setGate({ kind: "unavailable" });
      return;
    }
    try {
      const course =
        scopedRoute?.kind === "course"
          ? scopedRoute.course.summary
          : await runtime.client.getCourse(courseId);
      if (course.id !== courseId || course.tenant !== sessionTenant) {
        setGate({ kind: "unavailable" });
        return;
      }
      if (course.role !== "instructor" && course.role !== "administrator") {
        setGate({ kind: "denied" });
        return;
      }
      setGate({ kind: "allowed", course });
    } catch {
      setGate({ kind: "unavailable" });
    }
  }

  onMount(() => void checkCourseAccess());

  if (!allowedGlobalRole) return <AssignmentEditingDenied />;
  return (
    <Show
      when={gate().kind === "allowed" && courseId !== undefined && assignmentId !== undefined}
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
      <AssignmentEditorPage
        repository={createAssignmentEditorRepository(runtime.client)}
        courseId={courseId as CourseId}
        assignmentId={assignmentId as AssignmentId}
        tenant={sessionTenant}
      />
    </Show>
  );
}
