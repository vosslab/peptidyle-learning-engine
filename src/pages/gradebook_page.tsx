// gradebook_page.tsx - focused answer-free Instructor Gradebook.

import { useParams } from "@solidjs/router";
import { For, Show, createResource, type JSX } from "solid-js";

import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseGradebook } from "../api/live_gradebook";
import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceReference } from "../navigation/public_route";
import { formatPointScore } from "../score_format";
import "./instructor_data_tables.css";

function progressLabel(completion: "inProgress" | "completed" | null): string {
  if (completion === "completed") return "Completed and scored";
  if (completion === "inProgress") return "In progress";
  return "Not started";
}

function GradebookEvidence(props: { readonly gradebook: CourseGradebook }): JSX.Element {
  return (
    <Show
      when={props.gradebook.studentWork.length > 0}
      fallback={
        <section class="gradebook-empty" aria-label="No active Student Work">
          <h2>No active Students yet</h2>
          <p>Student Assignment progress will appear here as answer-free course evidence.</p>
        </section>
      }
    >
      <div class="gradebook-table-wrap" role="region" aria-label="Gradebook evidence">
        <table class="gradebook-table">
          <thead>
            <tr>
              <th scope="col">Roster ID</th>
              <th scope="col">Assignment</th>
              <th scope="col">Progress</th>
              <th scope="col">Graded Questions</th>
              <th scope="col">Score</th>
            </tr>
          </thead>
          <tbody>
            <For each={props.gradebook.studentWork}>
              {(work) => (
                <tr>
                  <td>{work.rosterId}</td>
                  <td>{work.assignmentReference}</td>
                  <td>{progressLabel(work.assignmentAttemptCompletion)}</td>
                  <td>
                    {work.gradedQuestionCount} of {work.questionCount}
                  </td>
                  <td>
                    {work.assignmentAttemptCompletion === null
                      ? "-"
                      : formatPointScore(work.pointsEarned, work.pointsPossible)}
                  </td>
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
  const [gradebook] = createResource(() => props.course, runtime.client.getCourseGradebook);
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

/** Loads the server-authorized Gradebook for the exact Course Instance route reference. */
export function GradebookPage(): JSX.Element {
  const params = useParams();
  // ASVS 2.2.1/8.3.1: validate the locator here; the server retains authorization.
  const course = (): ReturnType<typeof parseCourseInstanceReference> =>
    parseCourseInstanceReference(params["courseRef"] ?? "");
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
      {(loadedCourse) => <GradebookCoursePage course={loadedCourse} />}
    </Show>
  );
}
