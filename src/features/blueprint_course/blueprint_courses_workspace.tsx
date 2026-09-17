// Live discovery for reusable Blueprint Courses.
import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createSignal, onMount, type JSX } from "solid-js";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import { CourseClassificationSummary } from "../../components/course_classification_summary";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import { BlueprintCourseCreateDialog } from "./blueprint_course_create_dialog";
import {
  appendBlueprintCoursePage,
  blueprintCourseContinuationPresentation,
} from "./blueprint_course_model";
import type { BlueprintCoursesWorkspaceProps } from "./blueprint_course_workspace_types";
import "./blueprint_course.css";

type LoadState = "loading" | "ready" | "error";
type NoticeKind = "status" | "alert";

interface Notice {
  readonly kind: NoticeKind;
  readonly text: string;
}

function referencePath(reference: string): string {
  return `/blueprint-courses/${encodeURIComponent(reference)}`;
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof BlueprintCourseConflictError) {
    return "Blueprint Course content changed in another editor. Reload before saving.";
  }
  if (error instanceof ApiRequestError) {
    if (error.status === 401)
      return "Your session ended. Sign in again, then return to this Blueprint Course.";
    if (error.status === 403 || error.status === 404)
      return "This Blueprint Course is unavailable for your Account.";
  }
  return error instanceof Error && error.message.length > 0 ? error.message : fallback;
}

export function BlueprintCoursesWorkspace(props: BlueprintCoursesWorkspaceProps): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [courses, setCourses] = createSignal<ReadonlyArray<BlueprintCourseSummaryView>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [includeArchived, setIncludeArchived] = createSignal(false);
  const [loadingMore, setLoadingMore] = createSignal(false);
  const [continuationFailed, setContinuationFailed] = createSignal(false);
  const [creating, setCreating] = createSignal(false);
  const [sort, setSort] = createSignal<"name" | "adoptions" | "students">("name");
  const sortedCourses = (): ReadonlyArray<BlueprintCourseSummaryView> =>
    [...courses()].sort((left, right) => {
      const popularity =
        sort() === "adoptions"
          ? right.total_adoptions - left.total_adoptions
          : sort() === "students"
            ? right.total_students_ever_enrolled - left.total_students_ever_enrolled
            : 0;
      return (
        popularity ||
        left.long_name.localeCompare(right.long_name) ||
        left.reference.localeCompare(right.reference)
      );
    });
  const [notice, setNotice] = createSignal<Notice>({
    kind: "status",
    text: "Loading Blueprint Courses.",
  });
  let createTrigger: HTMLButtonElement | undefined;
  let discoveryRequest = 0;
  async function load(includeArchivedCourses = includeArchived()): Promise<void> {
    const request = ++discoveryRequest;
    setState("loading");
    try {
      const page = await props.client.listBlueprintCourses(
        undefined,
        50,
        includeArchivedCourses ? true : undefined,
      );
      if (request !== discoveryRequest) return;
      setCourses(page.items);
      setCursor(page.nextCursor);
      setContinuationFailed(false);
      setState("ready");
      setNotice({
        kind: "status",
        text: "Choose a Blueprint Course to inspect or create a new reusable course structure.",
      });
    } catch (error: unknown) {
      if (request !== discoveryRequest) return;
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
    const request = ++discoveryRequest;
    const includeArchivedCourses = includeArchived();
    setLoadingMore(true);
    try {
      const page = await props.client.listBlueprintCourses(
        nextCursor,
        50,
        includeArchivedCourses ? true : undefined,
      );
      if (request !== discoveryRequest) return;
      setCourses((current) => appendBlueprintCoursePage(current, page.items));
      setCursor(page.nextCursor);
      setContinuationFailed(false);
    } catch (error: unknown) {
      if (request !== discoveryRequest) return;
      setContinuationFailed(true);
      setNotice({
        kind: "alert",
        text: errorMessage(error, "More Blueprint Courses could not load. Try again when ready."),
      });
    } finally {
      if (request === discoveryRequest) setLoadingMore(false);
    }
  }

  function changeIncludeArchived(next: boolean): void {
    setIncludeArchived(next);
    setCourses([]);
    setCursor(null);
    setLoadingMore(false);
    setContinuationFailed(false);
    void load(next);
  }

  onMount(() => void load());
  const continuation = (): ReturnType<typeof blueprintCourseContinuationPresentation> =>
    blueprintCourseContinuationPresentation(cursor() !== null, continuationFailed());

  return (
    <main class="page blueprint-course-workspace" data-route-surface="blueprintCourses">
      <header class="blueprint-course-page-heading">
        <p class="eyebrow">Blueprint Courses</p>
        <h1>Build reusable course structure</h1>
        <Show when={props.proposalClient}>
          <A href="/blueprint-change-proposals">My Change Proposals</A>
        </Show>
        <p class="page-lede">
          Blueprint Courses contain reusable modules and assessments, with no Students or delivery
          dates.
        </p>
      </header>
      <p class="blueprint-course-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <section class="blueprint-course-card" aria-labelledby="blueprint-courses-heading">
        <div class="blueprint-course-section-heading">
          <div>
            <h2 id="blueprint-courses-heading">Available Blueprint Courses</h2>
            <p>Every active Instructor can inspect reusable question structure.</p>
          </div>
          <div class="blueprint-course-inline-actions">
            <label class="blueprint-course-archive-filter">
              <input
                type="checkbox"
                checked={includeArchived()}
                onChange={(event) => changeIncludeArchived(event.currentTarget.checked)}
              />
              Include Archived
            </label>
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
        </div>
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
            <Show
              when={courses().length > 0}
              fallback={
                <p class="blueprint-course-empty-copy">
                  No Blueprint Courses are visible yet. Create the first one.
                </p>
              }
            >
              <label class="blueprint-course-sort">
                Sort Blueprint Courses
                <select
                  value={sort()}
                  onChange={(event) => {
                    const value = event.currentTarget.value;
                    if (value === "name" || value === "adoptions" || value === "students")
                      setSort(value);
                  }}
                >
                  <option value="name">Name</option>
                  <option value="adoptions">Total adoptions</option>
                  <option value="students">Students ever enrolled</option>
                </select>
              </label>
              <ul class="blueprint-course-summary-list">
                <For each={sortedCourses()}>
                  {(course) => (
                    <li>
                      <A href={referencePath(course.reference)}>
                        <strong>{course.long_name}</strong>
                        <span>
                          {course.total_adoptions.toLocaleString()} adoptions ·{" "}
                          {course.total_students_ever_enrolled.toLocaleString()} students ever
                          enrolled
                        </span>
                        <span>
                          {course.read_access === "blueprint_course_owner"
                            ? "You are the Blueprint Course Owner."
                            : "Inspect its reusable modules."}{" "}
                          Current Blueprint Revision {course.current_revision.revision}.
                        </span>
                      </A>
                      <CourseClassificationSummary value={course.classification} />
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
          </Match>
        </Switch>
      </section>
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

/** Loads one opaque Blueprint Course and lets its owner Save complete Revision content. */
