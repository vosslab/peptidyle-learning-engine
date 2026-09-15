// assessment_preview_page.tsx - Instructor-only, answer-free saved Assessment preview.

import { A, useParams } from "@solidjs/router";
import { For, Match, Show, Switch, createResource, type JSX } from "solid-js";

import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { CourseRouteView } from "../api/contracts";
import { ApiRequestError } from "../api/http_client";
import { useApplicationApi } from "../api/application_api";
import { courseRouteView } from "../features/course_appearance/course_theme_context";
import { parseAssessmentReference } from "../navigation/public_route";
import { useRouteScopeData } from "../ribbon/route_scope_context";
import "./assessment_preview_page.css";

type PreviewFailure = "unavailable" | "offline" | "error";

function failureState(error: unknown): PreviewFailure {
  if (
    error instanceof ApiRequestError &&
    (error.status === 401 || error.status === 403 || error.status === 404)
  )
    return "unavailable";
  if (error instanceof TypeError || !navigator.onLine) return "offline";
  return "error";
}

interface AssessmentPreviewContentProps {
  readonly course: CourseRouteView["summary"];
}

/** The authorized live client returns only saved title, instructions, and Question descriptions. */
function AssessmentPreviewContent(props: AssessmentPreviewContentProps): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  const assessment = (): AssessmentReference | undefined => {
    const reference = params["assessmentRef"];
    return reference === undefined ? undefined : (parseAssessmentReference(reference) ?? undefined);
  };
  const [preview, { refetch }] = createResource(
    () => (props.course.role === "instructor" ? assessment() : undefined),
    (reference) => runtime.client.getLiveAssessmentPreview(props.course.reference, reference),
  );
  const failure = (): PreviewFailure | undefined => {
    if (props.course.role !== "instructor" || assessment() === undefined) return "unavailable";
    return preview.error === undefined ? undefined : failureState(preview.error);
  };

  return (
    <section class="page assessment-preview-page" data-route-surface="assessmentPreview">
      <p class="preview-only-cue">Preview only - no Student work or grades are created.</p>
      <p class="eyebrow">Instructor delivery inspection</p>
      <h1>Assessment delivery check</h1>
      <Show when={assessment()}>
        {(reference) => (
          <A
            class="quiet-link"
            href={`/instructor/courses/${props.course.reference}/assessments/${reference()}/properties`}
          >
            Return to Assessment Properties
          </A>
        )}
      </Show>
      <Switch>
        <Match when={preview.loading}>
          <p role="status">Loading assessment preview...</p>
        </Match>
        <Match when={failure() === "unavailable"}>
          <p role="alert">This assessment delivery check is unavailable.</p>
        </Match>
        <Match when={failure() === "offline"}>
          <p role="alert">The assessment preview is unavailable while offline.</p>
          <button type="button" onClick={() => void refetch()}>
            Reconnect and retry
          </button>
        </Match>
        <Match when={failure() === "error"}>
          <p role="alert">The assessment delivery check could not load.</p>
          <button class="primary-action" type="button" onClick={() => void refetch()}>
            Try again
          </button>
        </Match>
        <Match when={preview.state === "ready"}>
          <Show when={preview()}>
            {(loaded) => (
              <section class="preview-content" aria-labelledby="preview-assessment-title">
                <h2 id="preview-assessment-title">{loaded().title}</h2>
                <Show when={loaded().instructions.length > 0}>
                  <h3>Instructions</h3>
                  <p class="preview-instructions">{loaded().instructions}</p>
                </Show>
                <h3>Questions</h3>
                <Show
                  when={loaded().questions.length > 0}
                  fallback={<p>No Questions have been added to this Assessment.</p>}
                >
                  <ol class="preview-question-list" aria-label="Assessment Questions">
                    <For each={loaded().questions}>
                      {(question) => <li>{question.description}</li>}
                    </For>
                  </ol>
                </Show>
              </section>
            )}
          </Show>
        </Match>
      </Switch>
    </section>
  );
}

/** Content owns its deferred course boundary; the persistent shell never gates this loader. */
export function AssessmentPreviewPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const course = (): CourseRouteView["summary"] | undefined => {
    const data = routeData();
    return data?.kind === "course" ? courseRouteView(data).summary : undefined;
  };
  return (
    <Show
      when={course()}
      keyed
      fallback={
        <section class="page assessment-preview-page" data-route-surface="assessmentPreview">
          <p class="loading-state" role="status">
            Loading assessment delivery check...
          </p>
        </section>
      }
    >
      {(loadedCourse) => <AssessmentPreviewContent course={loadedCourse} />}
    </Show>
  );
}
