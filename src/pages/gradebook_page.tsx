// gradebook_page.tsx - focused M15 answer-free Instructor Gradebook.

import { For, Show, createResource, type JSX } from "solid-js";

import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { LiveDemoGradebook } from "../api/live_gradebook";
import { useApplicationApi } from "../api/application_api";
import { courseRouteView } from "../features/course_appearance/course_theme_context";
import { useRouteScopeData } from "../ribbon/route_scope_context";
import { formatPointScore } from "../score_format";
import "./instructor_data_tables.css";

function GradebookEvidence(props: { readonly gradebook: LiveDemoGradebook }): JSX.Element {
  return (
    <Show
      when={props.gradebook.gradedStudentWork.length > 0}
      fallback={
        <section class="gradebook-empty" aria-label="No graded Student Work">
          <h2>No graded Student Work yet</h2>
          <p>Completed grading will appear here as answer-free course evidence.</p>
        </section>
      }
    >
      <div class="gradebook-table-wrap" role="region" aria-label="Gradebook evidence">
        <table class="gradebook-table">
          <thead>
            <tr>
              <th scope="col">Roster ID</th>
              <th scope="col">Assignment</th>
              <th scope="col">Graded Questions</th>
              <th scope="col">Score</th>
            </tr>
          </thead>
          <tbody>
            <For each={props.gradebook.gradedStudentWork}>
              {(work) => (
                <tr>
                  <td>{work.rosterId}</td>
                  <td>{work.assignmentReference}</td>
                  <td>{work.gradedQuestionCount}</td>
                  <td>{formatPointScore(work.pointsEarned, work.pointsPossible)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </Show>
  );
}

function GradebookCoursePage(props: { readonly course: CourseInstanceReference }): JSX.Element {
  const runtime = useApplicationApi();
  const [gradebook] = createResource(() => props.course, runtime.client.getLiveDemoGradebook);
  return (
    <section class="page gradebook-page" data-route-surface="gradebook">
      <p class="eyebrow">Course progress</p>
      <h1>Gradebook</h1>
      <p class="page-lede">
        Immutable grading evidence for this Course Instance. Student responses, Question answers,
        Answer Keys, source content, and grader internals are not displayed here.
      </p>
      <Show when={gradebook.loading}>
        <p class="loading-state" role="status">
          Loading Gradebook evidence...
        </p>
      </Show>
      <Show when={gradebook.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Gradebook unavailable</h2>
          <p>This Course Instance is not available through your current Instructor access.</p>
        </section>
      </Show>
      <Show when={gradebook()}>{(loaded) => <GradebookEvidence gradebook={loaded()} />}</Show>
    </section>
  );
}

/** Resolves the course reference through the existing current-course route scope. */
export function GradebookPage(): JSX.Element {
  const scopedRoute = useRouteScopeData();
  const course = (): ReturnType<typeof courseRouteView>["summary"] | undefined => {
    const data = scopedRoute();
    return data?.kind === "course" ? courseRouteView(data).summary : undefined;
  };
  return (
    <Show
      when={course()}
      keyed
      fallback={
        <section class="page gradebook-page" data-route-surface="gradebook">
          <section class="route-error" role="alert">
            <h1>Gradebook unavailable</h1>
            <p>Return to your course list, then open the Gradebook again.</p>
          </section>
        </section>
      }
    >
      {(loadedCourse) => <GradebookCoursePage course={loadedCourse.reference} />}
    </Show>
  );
}
