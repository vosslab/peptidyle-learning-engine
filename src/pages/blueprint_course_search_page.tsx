// Instructor-only discovery of available Public Blueprint Courses.
import { Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseClient, BlueprintCourseListSort } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame";
import {
  RecordPageControls,
  type RecordPageSize,
} from "../components/record_list/record_page_controls";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import { RecordSortControl } from "../components/record_list/record_sort_control";
import { ApiProtocolError, ApiRequestError } from "../api/http_client";
import {
  BlueprintSearchClassification,
  emptyBlueprintClassificationSearch,
} from "./blueprint_course_search_classification";
import {
  BLUEPRINT_SEARCH_RETURN_PARAMETER,
  blueprintSearchReturnPath,
  createBlueprintSearchReturnToken,
  saveBlueprintSearchReturnState,
  takeBlueprintSearchReturnState,
  type BlueprintSearchSnapshot as SearchSnapshot,
} from "./blueprint_course_search_return_state";
import "../features/blueprint_course/blueprint_course.css";

export interface PublicBlueprintSearchPageProps {
  readonly client: Pick<BlueprintCourseClient, "listBlueprintCourses"> &
    ContentClassificationClient;
}

interface BlueprintPageRequest {
  readonly snapshot: SearchSnapshot;
  readonly cursor: string | undefined;
  readonly previousCursors: ReadonlyArray<string | undefined>;
  readonly pageSize: RecordPageSize;
  readonly focusResults: boolean;
}

function emptySearch(): SearchSnapshot {
  return {
    query: "",
    promotedOnly: false,
    classification: emptyBlueprintClassificationSearch(),
    classificationDescription: "",
    sort: "name",
  };
}

function blueprintCoursePath(blueprintCourseId: string, returnToken: string): string {
  return `/blueprint-courses/${encodeURIComponent(blueprintCourseId)}?${new URLSearchParams({
    [BLUEPRINT_SEARCH_RETURN_PARAMETER]: returnToken,
  }).toString()}`;
}

function searchErrorMessage(failure: unknown): string {
  if (failure instanceof ApiProtocolError) return failure.message;
  if (failure instanceof ApiRequestError && failure.status === 401)
    return "Your session ended. Sign in again, then return to Public Blueprint Search.";
  if (failure instanceof ApiRequestError && failure.status === 403)
    return "Public Blueprint Search requires an active Instructor Account.";
  return "Public Blueprint Courses could not load. Try again.";
}

function publicBlueprintContent(
  course: BlueprintCourseSummaryView,
  returnToken: string,
  resultLinks: Map<string, HTMLAnchorElement>,
  saveReturn: (event: MouseEvent, returnToken: string, linkKey: string) => void,
): RecordContent {
  return {
    title: course.long_name,
    description: course.short_name,
    details: [
      {
        kind: "text",
        label: "Current Blueprint Revision",
        value: course.current_revision_tuple.revisionNumber,
      },
      { kind: "text", label: "Adoptions", value: course.total_adoptions.toLocaleString() },
      {
        kind: "text",
        label: "Students ever enrolled",
        value: course.total_students_ever_enrolled.toLocaleString(),
      },
      { kind: "courseClassification", value: course.classification },
    ],
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open Blueprint",
        href: blueprintCoursePath(course.id, returnToken),
        primary: true,
        ref: (element) => resultLinks.set(`open:${course.id}`, element),
        onFollow: (event) => saveReturn(event, returnToken, `open:${course.id}`),
      },
    ],
  };
}

/** Local signals own filters and one server-returned page; the server owns public visibility. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  const sessionState = useSessionBootstrap().state();
  if (sessionState.kind !== "authenticated")
    throw new Error("Public Blueprint Search requires an authenticated session scope");
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
  const [currentCursor, setCurrentCursor] = createSignal<string | undefined>(
    returnState?.currentCursor,
  );
  const [previousCursors, setPreviousCursors] = createSignal<ReadonlyArray<string | undefined>>(
    returnState?.previousCursors ?? [],
  );
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [pageSize, setPageSize] = createSignal<RecordPageSize>(returnState?.pageSize ?? 50);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [pendingRequest, setPendingRequest] = createSignal<BlueprintPageRequest>();
  let generation = 0;
  let restoreFrame: number | undefined;
  let resultsStatus: HTMLParagraphElement | undefined;
  const resultLinks = new Map<string, HTMLAnchorElement>();
  const returnTokens = new Map<string, string>();

  function pageRequest(
    snapshot: SearchSnapshot,
    cursor: string | undefined,
    previous: ReadonlyArray<string | undefined>,
    selectedPageSize = pageSize(),
    focusResults = false,
  ): BlueprintPageRequest {
    return {
      snapshot,
      cursor,
      previousCursors: previous,
      pageSize: selectedPageSize,
      focusResults,
    };
  }
  function focusResults(): void {
    queueMicrotask(() => resultsStatus?.focus());
  }
  function collectionState(): RecordListState {
    if (loading()) return { kind: "loading", label: "Searching Public Blueprint Courses." };
    if (error() !== null)
      return {
        kind: "error",
        title: "Public Blueprint Courses unavailable",
        message: error()!,
        retry: (): void => {
          const target = pendingRequest();
          if (target !== undefined) void load(target);
        },
        retryLabel: "Retry search",
      };
    return { kind: "ready" };
  }
  async function load(target: BlueprintPageRequest): Promise<number | null> {
    const request = ++generation;
    setPendingRequest(target);
    setError(null);
    setLoading(true);
    resultLinks.clear();
    if (target.cursor === undefined) setCourses([]);
    try {
      const page = await props.client.listBlueprintCourses(
        target.cursor,
        target.pageSize,
        false,
        target.snapshot.query,
        true,
        target.snapshot.promotedOnly,
        target.snapshot.classification,
        target.snapshot.sort,
      );
      if (request !== generation) return null;
      setSubmitted(target.snapshot);
      setCourses(page.items);
      setCurrentCursor(target.cursor);
      setPreviousCursors(target.previousCursors);
      setNextCursor(page.nextCursor);
      setPageSize(target.pageSize);
      setPendingRequest(undefined);
      if (target.focusResults) focusResults();
      return request;
    } catch (failure: unknown) {
      if (request !== generation) return null;
      setCourses([]);
      setNextCursor(null);
      setError(searchErrorMessage(failure));
      return null;
    } finally {
      if (request === generation) setLoading(false);
    }
  }
  function submitSearch(): void {
    void load(
      pageRequest(
        {
          query: draft().trim(),
          promotedOnly: draftPromotedOnly(),
          classification: draftClassification(),
          classificationDescription: draftClassificationDescription(),
          sort: submitted().sort,
        },
        undefined,
        [],
        pageSize(),
        true,
      ),
    );
  }
  function resetSearch(): void {
    setDraft("");
    setDraftPromotedOnly(false);
    setDraftClassification(emptyBlueprintClassificationSearch());
    setDraftClassificationDescription("");
    void load(pageRequest(emptySearch(), undefined, [], pageSize(), true));
  }
  function previousPage(): void {
    const previous = previousCursors();
    if (previous.length === 0) return;
    void load(
      pageRequest(
        submitted(),
        previous[previous.length - 1],
        previous.slice(0, -1),
        pageSize(),
        true,
      ),
    );
  }
  function nextPage(): void {
    const next = nextCursor();
    if (next === null) return;
    void load(
      pageRequest(submitted(), next, [...previousCursors(), currentCursor()], pageSize(), true),
    );
  }
  function changeSort(sort: BlueprintCourseListSort): void {
    if (sort === submitted().sort) return;
    void load(pageRequest({ ...submitted(), sort }, undefined, [], pageSize(), true));
  }
  function changePageSize(next: RecordPageSize): void {
    if (next !== pageSize()) void load(pageRequest(submitted(), undefined, [], next, true));
  }
  function returnTokenFor(course: BlueprintCourseSummaryView): string {
    const existing = returnTokens.get(course.id);
    if (existing !== undefined) return existing;
    const token = createBlueprintSearchReturnToken();
    returnTokens.set(course.id, token);
    return token;
  }
  function saveReturn(event: MouseEvent, returnToken: string, linkKey: string): void {
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey ||
      loading()
    )
      return;
    saveBlueprintSearchReturnState(sessionScope, returnToken, {
      draft: {
        query: draft(),
        promotedOnly: draftPromotedOnly(),
        classification: draftClassification(),
        classificationDescription: draftClassificationDescription(),
        sort: submitted().sort,
      },
      submitted: submitted(),
      currentCursor: currentCursor(),
      previousCursors: previousCursors(),
      pageSize: pageSize(),
      scrollY: window.scrollY,
      linkKey,
    });
    history.replaceState(history.state, "", blueprintSearchReturnPath(returnToken));
  }
  onMount(() => {
    async function restore(): Promise<void> {
      const request = await load(
        pageRequest(
          returnState?.submitted ?? emptySearch(),
          returnState?.currentCursor,
          returnState?.previousCursors ?? [],
          returnState?.pageSize ?? 50,
        ),
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
            submitSearch();
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
        <p class="instructor-list__metadata">
          Applied search: {submitted().query === "" ? "all names" : `"${submitted().query}"`};{" "}
          {submitted().promotedOnly ? "Promoted only" : "all promotions"};{" "}
          {submitted().classificationDescription || "all classifications"};{" "}
          {submitted().sort === "adoptions"
            ? "most adoptions"
            : submitted().sort === "students"
              ? "most students"
              : "name"}
          .
        </p>
        <RecordSortControl
          label="Sort Public Blueprint Courses"
          options={[
            { value: "name", label: "Name" },
            { value: "adoptions", label: "Adoptions" },
            { value: "students", label: "Students ever enrolled" },
          ]}
          value={submitted().sort}
          disabled={loading()}
          onChange={changeSort}
        />
        <Show when={!loading() && error() === null}>
          <p
            role="status"
            tabindex="-1"
            ref={(element) => {
              resultsStatus = element;
            }}
          >
            {courses().length.toLocaleString()} Public Blueprint{" "}
            {courses().length === 1 ? "Course" : "Courses"} shown.
            {nextCursor() !== null ? " More results are available." : ""}
          </p>
        </Show>
        <RecordList
          ariaLabel="Available Public Blueprint Courses"
          emptyState={{
            title: "No available Public Blueprint Courses match.",
            message: "Try another name or classification, or clear the search.",
          }}
          recordId={(course) => course.id}
          content={(course) =>
            publicBlueprintContent(course, returnTokenFor(course), resultLinks, saveReturn)
          }
          rows={courses()}
          state={collectionState()}
        />
        <RecordPageControls
          ariaLabel="Public Blueprint Course pages"
          hasPrevious={previousCursors().length > 0}
          hasNext={nextCursor() !== null}
          loading={loading()}
          disabled={error() !== null}
          onPrevious={previousPage}
          onNext={nextPage}
          pageSize={pageSize()}
          onPageSizeChange={changePageSize}
        />
      </section>
    </PageFrame>
  );
}
