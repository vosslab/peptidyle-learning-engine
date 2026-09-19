// assessment_workspace_create_page.tsx - deliberate Assessment creation for the workspace.

import { A, useNavigate, useParams } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import {
  ASSESSMENT_TYPE_OPTIONS,
  assessmentTypePresentation,
} from "../../assessment_type_presentation";
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { AssessmentTemplate } from "../../../generated/api/AssessmentTemplate";
import { useApplicationApi } from "../../api/application_api";
import type { LiveAssessmentWorkspaceResponse } from "../../api/assessment_release";
import { useSessionBootstrap } from "../../auth/session_context";
import { courseRouteView } from "../../features/course_appearance/course_theme_context";
import {
  assessmentRouteId,
  parseCourseInstanceId,
  type CourseInstanceRouteId,
} from "../../navigation/public_route";
import { useRouteScopeData } from "../../ribbon/route_scope_context";

import {
  assessmentWorkspaceCreateErrorMessage,
  assessmentWorkspaceTemplateCreateErrorMessage,
  createdAssessmentQuestionsPath,
  selectedAssessmentType,
} from "./assessment_workspace_create_model";
import "./assessment_workspace.css";

type CreateState = "ready" | "saving" | "unavailable";
type CreationMethod = "manual" | "template";
type TemplateListState = "loading" | "ready" | "error";

/** Creates one real Assessment before the Instructor begins the Questions workflow. */
export function AssessmentWorkspaceCreatePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const route = useRouteScopeData();
  const params = useParams();
  const navigate = useNavigate();
  const [title, setTitle] = createSignal("");
  const [assessmentType, setAssessmentType] = createSignal<AssessmentType>();
  const [creationMethod, setCreationMethod] = createSignal<CreationMethod>("manual");
  const [templates, setTemplates] = createSignal<ReadonlyArray<AssessmentTemplate>>([]);
  const [templateListState, setTemplateListState] = createSignal<TemplateListState>("loading");
  const [templateId, setTemplateId] = createSignal("");
  const [message, setMessage] = createSignal("");
  const [state, setState] = createSignal<CreateState>("ready");
  let titleInput: HTMLInputElement | undefined;
  const course = (): ReturnType<typeof courseRouteView>["summary"] | undefined =>
    route()?.kind === "course" ? courseRouteView(route()!).summary : undefined;
  const courseInstanceId = (): CourseInstanceRouteId | null =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
  const mayCreate = (): boolean => {
    const currentSession = session.state();
    const currentCourse = course();
    const id = courseInstanceId();
    return (
      currentSession.kind === "authenticated" &&
      currentSession.session.account.productRole === "instructor" &&
      currentCourse?.role === "instructor" &&
      id !== null &&
      currentCourse.id === id
    );
  };
  const selectedTemplate = (): AssessmentTemplate | undefined =>
    templates().find((template) => template.id === templateId());

  async function loadTemplates(): Promise<void> {
    setTemplateListState("loading");
    try {
      setTemplates(await applicationApi.client.listAssessmentTemplates());
      setTemplateListState("ready");
    } catch {
      setTemplateListState("error");
    }
  }

  async function createAssessment(): Promise<void> {
    const currentCourse = course();
    const id = courseInstanceId();
    if (currentCourse === undefined || id === null || !mayCreate()) {
      setState("unavailable");
      return;
    }
    if (title().trim() === "") {
      setMessage("Enter an Assessment Title to create the Assessment.");
      return;
    }
    if (creationMethod() === "template" && selectedTemplate() === undefined) {
      setMessage("Choose a Template before creating the Assessment.");
      return;
    }
    if (creationMethod() === "manual" && assessmentType() === undefined) {
      setMessage("Choose an Assessment Type before creating the Assessment.");
      return;
    }
    setState("saving");
    setMessage("");
    try {
      const created =
        creationMethod() === "template"
          ? await createAssessmentFromTemplate(id)
          : await createManualAssessment(id);
      navigate(
        createdAssessmentQuestionsPath(id, assessmentRouteId(created.workspace.id)),
        {
          replace: true,
        },
      );
    } catch {
      setMessage(
        creationMethod() === "template"
          ? assessmentWorkspaceTemplateCreateErrorMessage()
          : assessmentWorkspaceCreateErrorMessage(),
      );
      setState("ready");
      queueMicrotask(() => titleInput?.focus());
    }
  }

  async function createManualAssessment(
    courseInstanceId: CourseInstanceRouteId,
  ): Promise<LiveAssessmentWorkspaceResponse> {
    const selectedType = assessmentType();
    if (selectedType === undefined) {
      throw new Error("An Assessment Type is required for manual creation.");
    }
    return applicationApi.client.createLiveAssessment(courseInstanceId, {
      assessmentType: selectedType,
      title: title(),
      instructions: "",
    });
  }

  async function createAssessmentFromTemplate(
    courseInstanceId: CourseInstanceRouteId,
  ): Promise<LiveAssessmentWorkspaceResponse> {
    const template = selectedTemplate();
    if (template === undefined) {
      throw new Error("An Assessment Template is required for Template creation.");
    }
    return applicationApi.client.createAssessmentFromTemplate(courseInstanceId, {
      templateId: template.id,
      title: title(),
    });
  }

  onMount(() => void loadTemplates());

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
            Choose an Assessment Type and enter a title. Questions and Assessment Properties have
            their own focused steps next.
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
              disabled={state() === "saving"}
              onInput={(event) => {
                setTitle(event.currentTarget.value);
                setMessage("");
              }}
            />
          </label>
          <fieldset class="assessment-create-method" disabled={state() === "saving"}>
            <legend>How do you want to start?</legend>
            <label class="assessment-create-method-choice">
              <input
                type="radio"
                name="assessment-create-method"
                checked={creationMethod() === "manual"}
                onChange={() => {
                  setCreationMethod("manual");
                  setMessage("");
                }}
              />
              Start with a new Assessment
            </label>
            <label class="assessment-create-method-choice">
              <input
                type="radio"
                name="assessment-create-method"
                checked={creationMethod() === "template"}
                onChange={() => {
                  setCreationMethod("template");
                  setMessage("");
                }}
              />
              Use one of my Templates
            </label>
          </fieldset>
          <Show
            when={creationMethod() === "manual"}
            fallback={
              <section
                class="assessment-create-template"
                aria-labelledby="assessment-template-choice"
              >
                <h2 id="assessment-template-choice">Use one of my Templates</h2>
                <Show
                  when={templateListState() === "ready"}
                  fallback={
                    <Show
                      when={templateListState() === "error"}
                      fallback={<p class="assessment-editor-note">Loading your Templates...</p>}
                    >
                      <p class="inline-error" role="alert">
                        Your Templates could not be loaded. You can retry or start with a new
                        Assessment.
                      </p>
                      <button class="quiet-link" type="button" onClick={() => void loadTemplates()}>
                        Retry loading Templates
                      </button>
                    </Show>
                  }
                >
                  <Show
                    when={templates().length > 0}
                    fallback={
                      <p class="assessment-editor-note">
                        You do not have any Templates yet. Start with a new Assessment or create a
                        Template first.
                      </p>
                    }
                  >
                    <label class="assessment-editor-field" for="assessment-template">
                      Template
                      <select
                        id="assessment-template"
                        value={templateId()}
                        disabled={state() === "saving"}
                        required
                        onChange={(event) => {
                          setTemplateId(event.currentTarget.value);
                          setMessage("");
                        }}
                      >
                        <option value="">Choose a Template</option>
                        <For each={templates()}>
                          {(template) => (
                            <option value={template.id}>
                              {template.name} -{" "}
                              {assessmentTypePresentation(template.assessmentType).label}
                            </option>
                          )}
                        </For>
                      </select>
                    </label>
                    <Show when={selectedTemplate()}>
                      {(template) => (
                        <p class="assessment-editor-note">
                          This Assessment will use the{" "}
                          {assessmentTypePresentation(template().assessmentType).label} Type and
                          this Template's settings.
                        </p>
                      )}
                    </Show>
                  </Show>
                </Show>
              </section>
            }
          >
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
          </Show>
          <Show when={creationMethod() === "manual" && assessmentType()}>
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
            <A
              class="quiet-link"
              href={`/courses/${courseInstanceId()!}`}
              aria-disabled={state() === "saving"}
              onClick={(event) => {
                if (state() === "saving") event.preventDefault();
              }}
            >
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
