// assignment_workspace_create_page.tsx - title-only persisted draft creation for the assignment workspace.

import { A, useNavigate, useParams } from "@solidjs/router";
import { Show, createSignal, type JSX } from "solid-js";

import { useApiRuntime } from "../../api/runtime";
import { useSessionBootstrap } from "../../auth/session_context";
import { CourseManagementNav } from "../../components/course_management_nav";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../../features/course_appearance/course_theme_context";
import {
  assignmentRouteReference,
  parseCourseReference,
  type CourseRouteReference,
} from "../../navigation/public_route";

import {
  assignmentWorkspaceCreateErrorMessage,
  createdAssignmentQuestionsPath,
} from "./assignment_workspace_create_model";

type CreateState = "ready" | "saving" | "unavailable";

/** Creates one real Draft before the Instructor begins the Questions workflow. */
export function AssignmentWorkspaceCreatePage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const route = useCourseThemeRouteData();
  const params = useParams();
  const navigate = useNavigate();
  const [title, setTitle] = createSignal("");
  const [message, setMessage] = createSignal("");
  const [state, setState] = createSignal<CreateState>("ready");
  let titleInput: HTMLInputElement | undefined;
  const course = (): ReturnType<typeof courseRouteData>["summary"] | undefined =>
    route?.kind === "course" ? courseRouteData(route).summary : undefined;
  const courseReference = (): CourseRouteReference | null =>
    parseCourseReference(params["courseRef"] ?? "");
  const mayCreate = (): boolean => {
    const currentSession = session.state();
    const currentCourse = course();
    const reference = courseReference();
    return (
      currentSession.kind === "authenticated" &&
      currentSession.session.user.roles.some(
        (role) => role === "instructor" || role === "sysadmin",
      ) &&
      currentCourse?.role === "instructor" &&
      reference !== null &&
      currentCourse.reference === reference
    );
  };

  async function createDraft(): Promise<void> {
    const currentCourse = course();
    const reference = courseReference();
    if (currentCourse === undefined || reference === null || !mayCreate()) {
      setState("unavailable");
      return;
    }
    if (title().trim() === "") {
      setMessage("Enter an assignment title to create the draft.");
      return;
    }
    setState("saving");
    setMessage("");
    try {
      const created = await runtime.client.createAssignmentDraft(currentCourse.id, {
        title: title(),
      });
      if (created.courseId !== currentCourse.id) {
        setState("unavailable");
        return;
      }
      navigate(
        createdAssignmentQuestionsPath(reference, assignmentRouteReference(created.reference)),
        {
          replace: true,
        },
      );
    } catch {
      setMessage(assignmentWorkspaceCreateErrorMessage());
      setState("ready");
      queueMicrotask(() => titleInput?.focus());
    }
  }

  return (
    <section class="page assignment-workspace-create" data-route-surface="assignmentCreate">
      <Show
        when={mayCreate() && state() !== "unavailable"}
        fallback={
          <section class="route-error" role="alert">
            <p class="eyebrow">Instructor assignment workspace</p>
            <h1>This assignment workspace is unavailable</h1>
            <p>Return to a course you manage to create an assignment.</p>
            <A class="primary-link" href="/">
              Return to courses
            </A>
          </section>
        }
      >
        <CourseManagementNav courseReference={course()!.reference} active="newAssignment" />
        <header class="assignment-workspace-header">
          <p class="eyebrow">New assignment</p>
          <h1>Create an assignment draft</h1>
          <p class="page-lede">
            Start with a title. Questions and delivery policies have their own focused steps next.
          </p>
        </header>
        <form
          class="assignment-editor-panel"
          aria-busy={state() === "saving"}
          onSubmit={(event) => {
            event.preventDefault();
            void createDraft();
          }}
        >
          <label class="assignment-editor-field" for="assignment-draft-title">
            Assignment title
            <input
              id="assignment-draft-title"
              ref={(element) => (titleInput = element)}
              autofocus
              value={title()}
              onInput={(event) => {
                setTitle(event.currentTarget.value);
                setMessage("");
              }}
            />
          </label>
          <p class="assignment-editor-note">
            The draft is saved now. Add at least one question in the next step before publishing.
          </p>
          <div class="assignment-editor-actions">
            <button class="primary-action" type="submit" disabled={state() === "saving"}>
              {state() === "saving" ? "Creating assignment draft..." : "Create assignment draft"}
            </button>
            <A class="quiet-link" href={`/courses/${courseReference()!}`}>
              Return to assignments
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
