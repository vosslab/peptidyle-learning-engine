// teaching_operations_page.tsx - authorized course shell for the teaching team and retention.

import { Show, createSignal, onMount, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import { RetentionPanel } from "./teaching_operations/retention_panel";
import { SysadminInstructorApprovalPanel } from "./teaching_operations/sysadmin_instructor_approval_panel";
import { TeachingTeamPanel } from "./teaching_team_panel";

type PageState = "loading" | "ready" | "denied" | "unavailable";

/** Course-local shell for Instructor teaching authority and course retention. */
export function TeachingOperationsPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const scopedRoute = useCourseThemeRouteData();
  const course = scopedRoute?.kind === "course" ? courseRouteData(scopedRoute).summary : undefined;
  const [state, setState] = createSignal<PageState>("loading");
  function load(): void {
    if (course === undefined) {
      setState("unavailable");
      return;
    }
    if (course.role !== "instructor") {
      setState("denied");
      return;
    }
    setState("loading");
    setState("ready");
  }

  const mayExtendRetention = (): boolean => {
    const current = session.state();
    return current.kind === "authenticated" && current.session.account.role === "sysadmin";
  };

  onMount(() => void load());

  return (
    <section class="page teaching-operations-page" data-route-surface="teachingOperations">
      <p class="eyebrow">Instructor course settings</p>
      <h1>Teaching operations</h1>
      <p class="page-lede">Manage the teaching team and student-record retention.</p>
      <Show when={state() === "loading"}>
        <p role="status">Loading teaching operations...</p>
      </Show>
      <Show when={state() === "denied"}>
        <p role="alert">You do not manage this course.</p>
      </Show>
      <Show when={state() === "unavailable"}>
        <section class="route-error" role="alert">
          <p>Teaching operations are unavailable for this course.</p>
          <button type="button" onClick={() => void load()}>
            Retry
          </button>
        </section>
      </Show>
      <Show when={course !== undefined && state() === "ready"}>
        <div class="teaching-operations-hub">
          <TeachingTeamPanel courseId={course!.id} />
          <Show when={mayExtendRetention()}>
            <SysadminInstructorApprovalPanel runtime={runtime} />
          </Show>
          <RetentionPanel
            courseId={course!.id}
            courseReference={course!.reference}
            runtime={runtime}
            mayExtendRetention={mayExtendRetention()}
          />
        </div>
      </Show>
    </section>
  );
}
