// Instructor-only discovery of available Public Blueprint Courses.
import { A } from "@solidjs/router";
import { Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseClient } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import {
  BlueprintSearchClassification,
  emptyBlueprintClassificationSearch,
} from "./blueprint_course_search_classification";
import { ApiProtocolError, ApiRequestError } from "../api/http_client";
import { appendBlueprintCoursePage } from "../features/blueprint_course/blueprint_course_model";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame";
import { RecordList } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
import {
  BLUEPRINT_SEARCH_RETURN_PARAMETER,
  saveBlueprintSearchReturnState,
  takeBlueprintSearchReturnState,
  type BlueprintSearchSnapshot as SearchSnapshot,
} from "./blueprint_course_search_return_state";
import "../features/blueprint_course/blueprint_course.css";

export interface PublicBlueprintSearchPageProps {
  readonly client: Pick<BlueprintCourseClient, "listBlueprintCourses"> &
    ContentClassificationClient;
}

function emptySearch(): SearchSnapshot {
  return {
    query: "",
    promotedOnly: false,
    classification: emptyBlueprintClassificationSearch(),
    classificationDescription: "",
  };
}

function publicBlueprintRegions(
  resultLinks: Map<string, HTMLAnchorElement>,
  saveReturn: (event: MouseEvent, linkKey: string) => void,
): ReadonlyArray<RecordRegion<BlueprintCourseSummaryView>> {
  return [
    {
      id: "blueprint",
      role: "identity",
      priority: "required",
      width: "minmax(14rem, 1.4fr)",
      align: "start",
      content: (course) => (
        <>
          <A
            href={`/blueprint-courses/${encodeURIComponent(course.id)}`}
            ref={(element) => resultLinks.set(`name:${course.id}`, element)}
            onClick={(event) => saveReturn(event, `name:${course.id}`)}
          >
            {course.long_name}
          </A>
          <p class="instructor-list__metadata">
            {course.short_name} - Current Blueprint Revision{" "}
            {course.current_revision_tuple.revisionNumber}
          </p>
        </>
      ),
    },
    {
      id: "use",
      role: "metadata",
      priority: "medium",
      width: "minmax(12rem, 1fr)",
      align: "start",
      content: (course) => (
        <>
          {course.total_adoptions.toLocaleString()} adoptions -{" "}
          {course.total_students_ever_enrolled.toLocaleString()} students ever enrolled
        </>
      ),
    },
    {
      id: "actions",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (course) => (
        <A
          class="secondary-action"
          href={`/blueprint-courses/${encodeURIComponent(course.id)}`}
          aria-label={`Open ${course.long_name}`}
          ref={(element) => resultLinks.set(`open:${course.id}`, element)}
          onClick={(event) => saveReturn(event, `open:${course.id}`)}
        >
          Open Blueprint
        </A>
      ),
    },
  ];
}

/** Local signals own draft/submitted text and pagination; the server owns public visibility. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  const sessionState = useSessionBootstrap().state();
  if (sessionState.kind !== "authenticated") {
    throw new Error("Public Blueprint Search requires an authenticated session scope");
  }
  const sessionScope = sessionState.session;
  const returnState = takeBlueprintSearchReturnState(
    sessionScope,
    new URLSearchParams(window.location.search).get(BLUEPRINT_SEARCH_RETURN_PARAMETER),
  );
  const [draft, setDraft] = createSignal(returnState?.draft.query ?? "");
  const [draftPromotedOnly, setDraftPromotedOnly] = createSignal(
    returnState?.draft.promotedOnly ?? false,
  );
  const [draftClassification, setDraftClassification] = createSignal(
    returnState?.draft.classification ?? emptyBlueprintClassificationSearch(),
  );
  const [draftClassificationDescription, setDraftClassificationDescription] = createSignal(
    returnState?.draft.classificationDescription ?? "",
  );
  const [submitted, setSubmitted] = createSignal<SearchSnapshot>(
    returnState?.submitted ?? emptySearch(),
  );
  const [courses, setCourses] = createSignal<ReadonlyArray<BlueprintCourseSummaryView>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [loadingMore, setLoadingMore] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [continuationFailed, setContinuationFailed] = createSignal(false);
  let generation = 0;
  let pages = 0;
  let restoreFrame: number | undefined;
  const resultLinks = new Map<string, HTMLAnchorElement>();

  async function load(
    snapshot: SearchSnapshot,
    nextCursor?: string,
    restorePages = 1,
  ): Promise<number | null> {
    const request = ++generation;
    const append = nextCursor !== undefined;
    setError(null);
    setContinuationFailed(false);
    if (append) setLoadingMore(true);
    else {
      setSubmitted(snapshot);
      setCourses([]);
      pages = 0;
      resultLinks.clear();
      setCursor(null);
      setLoadingMore(false);
      setLoading(true);
    }
    try {
      let pageCursor = nextCursor;
      // ASVS 8.3.1: refetch each page; retained presentation state grants no access.
      for (let index = 0; index < restorePages; index += 1) {
        const page = await props.client.listBlueprintCourses(
          pageCursor,
          50,
          false,
          snapshot.query,
          true,
          snapshot.promotedOnly,
          snapshot.classification,
        );
        if (request !== generation) return null;
        setCourses((current) =>
          append || index > 0 ? appendBlueprintCoursePage(current, page.items) : page.items,
        );
        pages += 1;
        setCursor(page.nextCursor);
        if (page.nextCursor === null) break;
        pageCursor = page.nextCursor;
      }
      return request;
    } catch (failure: unknown) {
      if (request !== generation) return null;
      const denied =
        failure instanceof ApiRequestError && (failure.status === 401 || failure.status === 403);
      if (denied) {
        setCourses([]);
        setCursor(null);
        pages = 0;
        resultLinks.clear();
      }
      setContinuationFailed(!denied && (append || pages > 0));
      setError(
        failure instanceof ApiProtocolError
          ? failure.message
          : failure instanceof ApiRequestError && failure.status === 401
            ? "Your session ended. Sign in again, then return to Public Blueprint Search."
            : failure instanceof ApiRequestError && failure.status === 403
              ? "Public Blueprint Search requires an active Instructor Account."
              : "Public Blueprint Courses could not load. Try again.",
      );
      return null;
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

  function resetSearch(): void {
    setDraft("");
    setDraftPromotedOnly(false);
    setDraftClassification(emptyBlueprintClassificationSearch());
    setDraftClassificationDescription("");
    void load(emptySearch());
  }

  function saveReturn(event: MouseEvent, linkKey: string): void {
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey ||
      loading() ||
      pages === 0
    )
      return;
    const token = saveBlueprintSearchReturnState(sessionScope, {
      draft: {
        query: draft(),
        promotedOnly: draftPromotedOnly(),
        classification: draftClassification(),
        classificationDescription: draftClassificationDescription(),
      },
      submitted: submitted(),
      pages,
      scrollY: window.scrollY,
      linkKey,
    });
    const source = new URL(window.location.href);
    // ASVS 1.2.2: only an opaque token is encoded into this same-origin source URL.
    source.searchParams.set(BLUEPRINT_SEARCH_RETURN_PARAMETER, token);
    history.replaceState(history.state, "", source);
  }

  onMount(() => {
    async function restore(): Promise<void> {
      const request = await load(
        returnState?.submitted ?? emptySearch(),
        undefined,
        returnState?.pages ?? 1,
      );
      if (returnState === null || request === null || request !== generation) return;
      restoreFrame = requestAnimationFrame(() => {
        if (request !== generation) return;
        resultLinks.get(returnState.linkKey)?.focus({ preventScroll: true });
        window.scrollTo({ top: returnState.scrollY, behavior: "instant" });
      });
    }
    void restore();
  });
  onCleanup(() => {
    generation += 1;
    if (restoreFrame !== undefined) cancelAnimationFrame(restoreFrame);
  });

  return (
    <PageFrame
      contentClass="blueprint-course-workspace"
      routeSurface="publicBlueprintSearch"
      eyebrow="Blueprint Courses"
      title="Search Public Blueprint Courses"
      lede="Find reusable course structure by short or long name. Open a Blueprint Course to inspect it or create a Course Instance."
    >
      <section class="blueprint-public-search" aria-labelledby="public-blueprint-results-heading">
        <form
          role="search"
          class="blueprint-public-search__form"
          onSubmit={(event) => {
            event.preventDefault();
            void load({
              query: draft().trim(),
              promotedOnly: draftPromotedOnly(),
              classification: draftClassification(),
              classificationDescription: draftClassificationDescription(),
            });
          }}
        >
          <div class="blueprint-public-search__query-row">
            <label class="blueprint-public-search__query" for="public-blueprint-query">
              <span>Blueprint Course name</span>
              <input
                id="public-blueprint-query"
                type="search"
                value={draft()}
                onInput={(event) => setDraft(event.currentTarget.value)}
              />
            </label>
            <label>
              <input
                type="checkbox"
                checked={draftPromotedOnly()}
                onChange={(event) => setDraftPromotedOnly(event.currentTarget.checked)}
              />{" "}
              Promoted only
            </label>
          </div>
          <div class="blueprint-public-search__classification">
            <BlueprintSearchClassification
              client={props.client}
              value={draftClassification()}
              onChange={(classification, description) => {
                setDraftClassification(classification);
                setDraftClassificationDescription(description);
              }}
            />
          </div>
          <div class="blueprint-public-search__actions">
            <button class="primary-action" type="submit">
              Search
            </button>
            <button type="button" onClick={resetSearch}>
              Clear search
            </button>
          </div>
        </form>
        <h2 id="public-blueprint-results-heading">Available Public Blueprint Courses</h2>
        {/* ASVS 1.2.1: applied names remain escaped text, including during failure/retry. */}
        <p class="instructor-list__metadata">
          Applied search: {submitted().query === "" ? "all names" : `"${submitted().query}"`};{" "}
          {submitted().promotedOnly ? "Promoted only" : "all promotions"};{" "}
          {submitted().classificationDescription || "all classifications"}.
        </p>
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
            {courses().length === 1 ? "Course" : "Courses"} shown.
            {cursor() !== null ? " More results are available." : ""}
          </p>
          <Show
            when={courses().length > 0}
            fallback={
              <p class="blueprint-course-empty-copy">
                No available Public Blueprint Courses match. Try another name or classification, or
                clear the search.
              </p>
            }
          >
            <RecordList
              ariaLabel="Available Public Blueprint Courses"
              emptyState={{ title: "No available Public Blueprint Courses match." }}
              recordId={(course) => course.id}
              regions={publicBlueprintRegions(resultLinks, saveReturn)}
              rows={courses()}
              state={{ kind: "ready" }}
            />
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
    </PageFrame>
  );
}
