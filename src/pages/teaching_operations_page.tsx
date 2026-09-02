// teaching_operations_page.tsx - authorized course shell for the teaching team.

import { Show, createSignal, onMount, type JSX } from "solid-js";

import {
  courseRouteView,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import { TeachingTeamPanel } from "./teaching_team_panel";

type PageState = "loading" | "ready" | "denied" | "unavailable";

/** Course-local shell for Instructor teaching authority. */
export function TeachingOperationsPage(): JSX.Element {
  const scopedRoute = useCourseThemeRouteData();
  const course = scopedRoute?.kind === "course" ? courseRouteView(scopedRoute).summary : undefined;
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

  onMount(() => void load());

  return (
    <section class="page teaching-operations-page" data-route-surface="teachingOperations">
      <p class="eyebrow">Instructor course settings</p>
      <h1>Teaching operations</h1>
      <p class="page-lede">Manage the teaching team.</p>
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
        </div>
      </Show>
    </section>
  );
}
