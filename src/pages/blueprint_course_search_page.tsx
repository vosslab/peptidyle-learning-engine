// Instructor-only discovery of available Public Blueprint Courses.
import { A } from "@solidjs/router";
import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseClient } from "../api/blueprint_course";
import { ApiProtocolError, ApiRequestError } from "../api/http_client";
import { appendBlueprintCoursePage } from "../features/blueprint_course/blueprint_course_model";
import "../features/blueprint_course/blueprint_course.css";

export interface PublicBlueprintSearchPageProps {
  readonly client: Pick<BlueprintCourseClient, "listBlueprintCourses">;
}

/** Local signals own draft/submitted text and pagination; the server owns public visibility. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  const [draft, setDraft] = createSignal("");
  const [submitted, setSubmitted] = createSignal("");
  const [courses, setCourses] = createSignal<ReadonlyArray<BlueprintCourseSummaryView>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [loadingMore, setLoadingMore] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [continuationFailed, setContinuationFailed] = createSignal(false);
  let generation = 0;

  async function load(query: string, nextCursor?: string): Promise<void> {
    const request = ++generation;
    const append = nextCursor !== undefined;
    setError(null);
    setContinuationFailed(false);
    if (append) setLoadingMore(true);
    else {
      setSubmitted(query);
      setCourses([]);
      setCursor(null);
      setLoadingMore(false);
      setLoading(true);
    }
    try {
      const page = await props.client.listBlueprintCourses(nextCursor, 50, false, query, true);
      if (request !== generation) return;
      setCourses((current) =>
        append ? appendBlueprintCoursePage(current, page.items) : page.items,
      );
      setCursor(page.nextCursor);
    } catch (failure: unknown) {
      if (request !== generation) return;
      setContinuationFailed(append);
      setError(
        failure instanceof ApiProtocolError
          ? failure.message
          : failure instanceof ApiRequestError && failure.status === 401
            ? "Your session ended. Sign in again, then return to Public Blueprint Search."
            : failure instanceof ApiRequestError && failure.status === 403
              ? "Public Blueprint Search requires an active Instructor Account."
              : "Public Blueprint Courses could not load. Try again.",
      );
    } finally {
      if (request === generation) {
        setLoading(false);
        setLoadingMore(false);
      }
    }
  }

  function loadMore(): void {
    const next = cursor();
    if (next !== null && !loadingMore() && !loading()) void load(submitted(), next);
  }

  onMount(() => void load(""));
  onCleanup(() => {
    generation += 1;
  });

  return (
    <main class="page blueprint-course-workspace" data-route-surface="publicBlueprintSearch">
      <header class="blueprint-course-page-heading">
        <p class="eyebrow">Blueprint Courses</p>
        <h1>Search Public Blueprint Courses</h1>
        <p class="page-lede">
          Find reusable course structure by short or long name. Open a Blueprint Course to inspect
          it or create a Course Instance.
        </p>
      </header>
      <section class="blueprint-course-card" aria-labelledby="public-blueprint-results-heading">
        <form
          class="blueprint-course-inline-actions"
          role="search"
          onSubmit={(event) => {
            event.preventDefault();
            void load(draft().trim());
          }}
        >
          <label for="public-blueprint-query">Blueprint Course name</label>
          <input
            id="public-blueprint-query"
            type="search"
            value={draft()}
            onInput={(event) => setDraft(event.currentTarget.value)}
          />
          <button type="submit">Search</button>
        </form>
        <h2 id="public-blueprint-results-heading">Available Public Blueprint Courses</h2>
        <Show when={loading()}>
          <p role="status">Searching Public Blueprint Courses.</p>
        </Show>
        <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
        <Show when={!loading() && error() !== null && !continuationFailed()}>
          <button type="button" onClick={() => void load(submitted())}>
            Retry search
          </button>
        </Show>
        <Show when={!loading() && (error() === null || continuationFailed())}>
          <p role="status">
            {courses().length.toLocaleString()} Public Blueprint{" "}
            {courses().length === 1 ? "Course" : "Courses"} shown
            {submitted() === "" ? " for all names" : ` for "${submitted()}"`}.
            {cursor() !== null ? " More results are available." : ""}
          </p>
          <Show
            when={courses().length > 0}
            fallback={
              <p class="blueprint-course-empty-copy">
                No available Public Blueprint Courses match. Try another name or clear the search.
              </p>
            }
          >
            <ul class="instructor-list" style={{ margin: "0", padding: "0", "list-style": "none" }}>
              <For each={courses()}>
                {(course) => (
                  <li class="instructor-list__row">
                    <div class="instructor-list__identity">
                      {/* ASVS 1.2.1, 1.2.2: text nodes and encoded internal identities, never raw HTML. */}
                      <h3>
                        <A href={`/blueprint-courses/${encodeURIComponent(course.reference)}`}>
                          {course.long_name}
                        </A>
                      </h3>
                      <p class="instructor-list__metadata">
                        {course.short_name} - Current Blueprint Revision{" "}
                        {course.current_revision.revision}
                      </p>
                    </div>
                    <p class="instructor-list__metadata">
                      {course.total_adoptions.toLocaleString()} adoptions -{" "}
                      {course.total_students_ever_enrolled.toLocaleString()} students ever enrolled
                    </p>
                    <div class="instructor-list__actions">
                      <A
                        class="secondary-action"
                        href={`/blueprint-courses/${encodeURIComponent(course.reference)}`}
                        aria-label={`Open ${course.long_name}`}
                      >
                        Open Blueprint
                      </A>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <Show when={cursor() !== null}>
            <div class="blueprint-course-continuation">
              <button type="button" disabled={loadingMore()} onClick={loadMore}>
                {loadingMore()
                  ? "Loading..."
                  : continuationFailed()
                    ? "Retry loading more"
                    : "Load more Public Blueprint Courses"}
              </button>
            </div>
          </Show>
        </Show>
      </section>
    </main>
  );
}
