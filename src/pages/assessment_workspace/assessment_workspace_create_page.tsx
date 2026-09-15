// assessment_workspace_create_page.tsx - deliberate Assessment creation for the workspace.

import { A, useNavigate, useParams } from "@solidjs/router";
import { For, Show, createSignal, type JSX } from "solid-js";

import {
  ASSESSMENT_TYPE_OPTIONS,
  assessmentTypePresentation,
} from "../../assessment_type_presentation";
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import { useApplicationApi } from "../../api/application_api";
import { useSessionBootstrap } from "../../auth/session_context";
import { courseRouteView } from "../../features/course_appearance/course_theme_context";
import {
  assessmentRouteReference,
  parseCourseInstanceReference,
  type CourseInstanceRouteReference,
} from "../../navigation/public_route";
import { useRouteScopeData } from "../../ribbon/route_scope_context";

import {
  assessmentWorkspaceCreateErrorMessage,
  createdAssessmentQuestionsPath,
  selectedAssessmentType,
} from "./assessment_workspace_create_model";

type CreateState = "ready" | "saving" | "unavailable";

/** Creates one real Assessment before the Instructor begins the Questions workflow. */
export function AssessmentWorkspaceCreatePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const route = useRouteScopeData();
  const params = useParams();
  const navigate = useNavigate();
  const [title, setTitle] = createSignal("");
  const [assessmentType, setAssessmentType] = createSignal<AssessmentType>();
  const [message, setMessage] = createSignal("");
  const [state, setState] = createSignal<CreateState>("ready");
  let titleInput: HTMLInputElement | undefined;
  const course = (): ReturnType<typeof courseRouteView>["summary"] | undefined =>
    route()?.kind === "course" ? courseRouteView(route()!).summary : undefined;
  const courseReference = (): CourseInstanceRouteReference | null =>
    parseCourseInstanceReference(params["courseRef"] ?? "");
  const mayCreate = (): boolean => {
    const currentSession = session.state();
    const currentCourse = course();
    const reference = courseReference();
    return (
      currentSession.kind === "authenticated" &&
      currentSession.session.account.productRole === "instructor" &&
      currentCourse?.role === "instructor" &&
      reference !== null &&
      currentCourse.reference === reference
    );
  };

  async function createAssessment(): Promise<void> {
    const currentCourse = course();
    const reference = courseReference();
    if (currentCourse === undefined || reference === null || !mayCreate()) {
      setState("unavailable");
      return;
    }
    if (title().trim() === "") {
      setMessage("Enter an Assessment Title to create the Assessment.");
      return;
    }
    const selectedType = assessmentType();
    if (selectedType === undefined) {
      setMessage("Choose an Assessment Type before creating the Assessment.");
      return;
    }
    setState("saving");
    setMessage("");
    try {
      const created = await applicationApi.client.createLiveAssessment(reference, {
        assessmentType: selectedType,
        title: title(),
        instructions: "",
      });
      navigate(
        createdAssessmentQuestionsPath(
          reference,
          assessmentRouteReference(created.workspace.reference),
        ),
        {
          replace: true,
        },
      );
    } catch {
      setMessage(assessmentWorkspaceCreateErrorMessage());
      setState("ready");
      queueMicrotask(() => titleInput?.focus());
    }
  }

  return (
    <section class="page assessment-workspace-create" data-route-surface="assessmentCreate">
      <Show
        when={mayCreate() && state() !== "unavailable"}
        fallback={
          <section class="route-error" role="alert">
            <p class="eyebrow">Instructor assessment workspace</p>
            <h1>This assessment workspace is unavailable</h1>
            <p>Return to a course you manage to create an assessment.</p>
            <A class="primary-link" href="/">
              Return to courses
            </A>
          </section>
        }
      >
        <header class="assessment-workspace-header">
          <p class="eyebrow">New assessment</p>
          <h1>Create an Assessment</h1>
          <p class="page-lede">
            Choose an Assessment Type and enter a title. Questions and delivery policies have their
            own focused steps next.
          </p>
        </header>
        <form
          class="assessment-editor-panel"
          aria-busy={state() === "saving"}
          onSubmit={(event) => {
            event.preventDefault();
            void createAssessment();
          }}
        >
          <label class="assessment-editor-field" for="assessment-title">
            Assessment title
            <input
              id="assessment-title"
              ref={(element) => (titleInput = element)}
              autofocus
              value={title()}
              onInput={(event) => {
                setTitle(event.currentTarget.value);
                setMessage("");
              }}
            />
          </label>
          <label class="assessment-editor-field" for="assessment-type">
            Assessment Type
            <select
              id="assessment-type"
              value={assessmentType() ?? ""}
              disabled={state() === "saving"}
              required
              onInput={(event) => {
                const value = event.currentTarget.value;
                setAssessmentType(selectedAssessmentType(value));
                setMessage("");
              }}
            >
              <option value="">Choose an Assessment Type</option>
              <For each={ASSESSMENT_TYPE_OPTIONS}>
                {(option) => <option value={option.value}>{option.label}</option>}
              </For>
            </select>
          </label>
          <Show when={assessmentType()}>
            {(selectedType) => (
              <p class="assessment-editor-note">
                {assessmentTypePresentation(selectedType()).description}
              </p>
            )}
          </Show>
          <div class="assessment-editor-actions">
            <button class="primary-action" type="submit" disabled={state() === "saving"}>
              {state() === "saving" ? "Creating Assessment..." : "Create Assessment"}
            </button>
            <A class="quiet-link" href={`/courses/${courseReference()!}`}>
              Return to assessments
            </A>
          </div>
          <Show when={message()}>
            {(value) => (
              <p class="inline-error" role="alert">
                {value()}
              </p>
            )}
          </Show>
        </form>
      </Show>
    </section>
  );
}
