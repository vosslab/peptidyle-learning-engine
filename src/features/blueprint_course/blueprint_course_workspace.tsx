// Live discovery, inspection, and Blueprint Course Owner editing for reusable Blueprint Courses.

import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createSignal, onMount, type JSX } from "solid-js";

import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import type { BlueprintCourseClient, BlueprintCourseEtag } from "../../api/blueprint_course";
import type { QuestionPickerSource, QuestionPickerSourceRepository } from "../question_picker";
import { BlueprintCourseCreateDialog } from "./blueprint_course_create_dialog";
import {
  appendBlueprintCoursePage,
  blueprintCourseContinuationPresentation,
  replacementContentFromBlueprintCourse,
  validateReusableContent,
} from "./blueprint_course_model";
import { BlueprintAssignmentContentEditor } from "./blueprint_assignment_content_editor";
import "./blueprint_course.css";

type LoadState = "loading" | "ready" | "error";
type NoticeKind = "status" | "alert";

interface Notice {
  readonly kind: NoticeKind;
  readonly text: string;
}

interface LoadedBlueprintCourse {
  readonly view: BlueprintCourseView;
  readonly etag: BlueprintCourseEtag;
  readonly draft: ReplaceBlueprintCourseContentInput;
}

export interface BlueprintCoursesWorkspaceProps {
  readonly client: BlueprintCourseClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
}

export interface BlueprintCourseDetailWorkspaceProps extends BlueprintCoursesWorkspaceProps {
  readonly blueprintCourseRef: string;
}

function referencePath(reference: string): string {
  return `/blueprint-courses/${encodeURIComponent(reference)}`;
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiRequestError) {
    if (error.status === 401)
      return "Your session ended. Sign in again, then return to this Blueprint Course.";
    if (error.status === 403 || error.status === 404)
      return "This Blueprint Course is unavailable for your Account.";
  }
  return error instanceof Error && error.message.length > 0 ? error.message : fallback;
}

/** Lists every Blueprint Course available to the current active Instructor. */
export function BlueprintCoursesWorkspace(props: BlueprintCoursesWorkspaceProps): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [courses, setCourses] = createSignal<ReadonlyArray<BlueprintCourseSummaryView>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [loadingMore, setLoadingMore] = createSignal(false);
  const [continuationFailed, setContinuationFailed] = createSignal(false);
  const [creating, setCreating] = createSignal(false);
  const [notice, setNotice] = createSignal<Notice>({
    kind: "status",
    text: "Loading Blueprint Courses.",
  });
  let createTrigger: HTMLButtonElement | undefined;

  async function load(): Promise<void> {
    setState("loading");
    try {
      const page = await props.client.listBlueprintCourses(undefined, 50);
      setCourses(page.items);
      setCursor(page.nextCursor);
      setContinuationFailed(false);
      setState("ready");
      setNotice({
        kind: "status",
        text: "Choose a Blueprint Course to inspect or create a new reusable course structure.",
      });
    } catch (error: unknown) {
      setState("error");
      setNotice({
        kind: "alert",
        text: errorMessage(error, "Blueprint Courses could not load. Try again."),
      });
    }
  }

  async function loadMore(): Promise<void> {
    const nextCursor = cursor();
    if (nextCursor === null || loadingMore()) return;
    setLoadingMore(true);
    try {
      const page = await props.client.listBlueprintCourses(nextCursor, 50);
      setCourses((current) => appendBlueprintCoursePage(current, page.items));
      setCursor(page.nextCursor);
      setContinuationFailed(false);
    } catch (error: unknown) {
      setContinuationFailed(true);
      setNotice({
        kind: "alert",
        text: errorMessage(error, "More Blueprint Courses could not load. Try again when ready."),
      });
    } finally {
      setLoadingMore(false);
    }
  }

  onMount(() => void load());
  const continuation = (): ReturnType<typeof blueprintCourseContinuationPresentation> =>
    blueprintCourseContinuationPresentation(cursor() !== null, continuationFailed());

  return (
    <main class="page blueprint-course-workspace" data-route-surface="blueprintCourses">
      <header class="blueprint-course-page-heading">
        <p class="eyebrow">Blueprint Courses</p>
        <h1>Build reusable course structure</h1>
        <p class="page-lede">
          Blueprint Courses contain reusable modules and assignments, with no Students or delivery
          dates.
        </p>
      </header>
      <p class="blueprint-course-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading Blueprint Courses.</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => void load()}>
            Retry loading Blueprint Courses
          </button>
        </Match>
        <Match when={state() === "ready"}>
          <section class="blueprint-course-card" aria-labelledby="blueprint-courses-heading">
            <div class="blueprint-course-section-heading">
              <div>
                <h2 id="blueprint-courses-heading">Available Blueprint Courses</h2>
                <p>Every active Instructor can inspect reusable question structure.</p>
              </div>
              <button
                type="button"
                onClick={(event) => {
                  createTrigger = event.currentTarget;
                  setCreating(true);
                }}
              >
                Create Blueprint Course
              </button>
            </div>
            <Show
              when={courses().length > 0}
              fallback={
                <p class="blueprint-course-empty-copy">
                  No Blueprint Courses are visible yet. Create the first one.
                </p>
              }
            >
              <ul class="blueprint-course-summary-list">
                <For each={courses()}>
                  {(course) => (
                    <li>
                      <A href={referencePath(course.reference)}>
                        <strong>{course.title}</strong>
                        <span>
                          {course.read_access === "blueprint_course_owner"
                            ? "You are the Blueprint Course Owner."
                            : "Inspect its reusable modules."}{" "}
                          Revision {course.revision}.
                        </span>
                      </A>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <Show when={continuation().visible}>
              <div class="blueprint-course-continuation">
                <p role={continuationFailed() ? "alert" : "status"}>
                  {continuationFailed()
                    ? "More Blueprint Courses are available. Retry when ready."
                    : "More Blueprint Courses are available."}
                </p>
                <button type="button" disabled={loadingMore()} onClick={() => void loadMore()}>
                  {loadingMore() ? "Loading..." : continuation().action}
                </button>
              </div>
            </Show>
          </section>
        </Match>
      </Switch>
      <Show when={creating()}>
        <BlueprintCourseCreateDialog
          client={props.client}
          pickerRepository={props.pickerRepository}
          pickerSources={props.pickerSources}
          onClose={() => {
            setCreating(false);
            queueMicrotask(() => createTrigger?.focus());
          }}
          onFailure={(text) => setNotice({ kind: "alert", text })}
        />
      </Show>
    </main>
  );
}

/** Loads one BP-* Blueprint Course and exposes its exact reusable structure. */
export function BlueprintCourseDetailWorkspace(
  props: BlueprintCourseDetailWorkspaceProps,
): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [current, setCurrent] = createSignal<LoadedBlueprintCourse>();
  const [notice, setNotice] = createSignal<Notice>({
    kind: "status",
    text: "Loading Blueprint Course.",
  });
  const [saving, setSaving] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);

  async function load(keepDraft: boolean): Promise<void> {
    if (!/^BP-[1-9][0-9]*$/u.test(props.blueprintCourseRef)) {
      setState("error");
      setNotice({ kind: "alert", text: "Blueprint Course references begin with BP-." });
      return;
    }
    setState("loading");
    try {
      const result = await props.client.getBlueprintCourse(props.blueprintCourseRef);
      const prior = current();
      setCurrent({
        view: result.blueprintCourse,
        etag: result.etag,
        draft:
          keepDraft && prior !== undefined
            ? prior.draft
            : replacementContentFromBlueprintCourse(result.blueprintCourse),
      });
      if (!keepDraft) setDirty(false);
      setConflict(false);
      setState("ready");
      setNotice({
        kind: "status",
        text:
          result.blueprintCourse.read_access === "blueprint_course_owner"
            ? "Blueprint Course loaded. Update its Blueprint Assignments deliberately."
            : "Blueprint Course loaded. Inspect its answer-free reusable structure.",
      });
    } catch (error: unknown) {
      setState("error");
      setNotice({
        kind: "alert",
        text: errorMessage(error, "This Blueprint Course could not load. Try again."),
      });
    }
  }

  function changeDraft(next: ReplaceBlueprintCourseContentInput, text: string): void {
    const loaded = current();
    if (loaded === undefined) return;
    setCurrent({ ...loaded, draft: next });
    setDirty(true);
    setNotice({ kind: "status", text });
  }

  function changeAssignment(
    moduleIndex: number,
    assignmentIndex: number,
    content: import("../../../generated/api/BlueprintAssignmentContentInput").BlueprintAssignmentContentInput,
    text: string,
  ): void {
    const loaded = current();
    const module = loaded?.draft.modules[moduleIndex];
    const assignment = module?.assignments[assignmentIndex];
    if (loaded === undefined || module === undefined || assignment === undefined) return;
    const modules = [...loaded.draft.modules];
    const assignments = [...module.assignments];
    assignments[assignmentIndex] = { ...assignment, content };
    modules[moduleIndex] = { ...module, assignments };
    changeDraft({ ...loaded.draft, modules }, text);
  }

  async function save(): Promise<void> {
    const loaded = current();
    if (loaded === undefined) return;
    for (const module of loaded.draft.modules) {
      for (const assignment of module.assignments) {
        const validation = validateReusableContent(assignment.content);
        if (!validation.valid) {
          setNotice({
            kind: "alert",
            text: validation.message ?? "Review this Blueprint Course before saving.",
          });
          return;
        }
      }
    }
    setSaving(true);
    try {
      const saved = await props.client.replaceBlueprintCourse(
        loaded.view.reference,
        loaded.draft,
        loaded.etag,
      );
      setCurrent({
        view: saved.blueprintCourse,
        etag: saved.etag,
        draft: replacementContentFromBlueprintCourse(saved.blueprintCourse),
      });
      setDirty(false);
      setConflict(false);
      setNotice({ kind: "status", text: "Blueprint Course saved with a new immutable revision." });
    } catch (error: unknown) {
      setConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "Blueprint Course could not save. Your local draft remains available.",
        ),
      });
    } finally {
      setSaving(false);
    }
  }

  onMount(() => void load(false));
  return (
    <main class="page blueprint-course-workspace" data-route-surface="blueprintCourseDetail">
      <A class="quiet-link" href="/blueprint-courses">
        Return to Blueprint Courses
      </A>
      <p class="blueprint-course-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading Blueprint Course.</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => void load(false)}>
            Retry loading Blueprint Course
          </button>
        </Match>
        <Match when={state() === "ready" && current()} keyed>
          {(loaded) => (
            <section class="blueprint-course-detail-editor">
              <header class="blueprint-course-page-heading">
                <p class="eyebrow">Blueprint Course</p>
                <h1>{loaded.view.title}</h1>
                <p class="page-lede">
                  Reusable course structure without Students, deadlines, or course delivery
                  settings.
                </p>
              </header>
              <Show when={loaded.view.read_access === "blueprint_course_owner"}>
                <footer class="blueprint-course-save-actions blueprint-course-detail-actions">
                  <button type="button" disabled={saving()} onClick={() => void save()}>
                    {saving() ? "Saving..." : "Save Blueprint Course"}
                  </button>
                  <Show when={dirty() || conflict()}>
                    <button type="button" class="quiet-action" onClick={() => void load(false)}>
                      {conflict() ? "Reload current version" : "Discard local changes"}
                    </button>
                  </Show>
                </footer>
              </Show>
              <div class="blueprint-course-editor-content">
                <For each={loaded.draft.modules}>
                  {(module, moduleIndex) => (
                    <section class="blueprint-course-module">
                      <h2>{module.label}</h2>
                      <For each={module.assignments}>
                        {(assignment, assignmentIndex) => (
                          <section class="blueprint-course-content-card">
                            <BlueprintAssignmentContentEditor
                              content={assignment.content}
                              editable={loaded.view.read_access === "blueprint_course_owner"}
                              pickerRepository={props.pickerRepository}
                              pickerSources={props.pickerSources}
                              onChange={(content, text) =>
                                changeAssignment(moduleIndex(), assignmentIndex(), content, text)
                              }
                            />
                          </section>
                        )}
                      </For>
                    </section>
                  )}
                </For>
              </div>
            </section>
          )}
        </Match>
      </Switch>
    </main>
  );
}
