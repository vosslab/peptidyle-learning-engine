// Instructor-only discovery of available Public Blueprint Courses.
import { Show, createSignal, onCleanup, type JSX } from "solid-js";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseClient, BlueprintCourseListSort } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame";
import {
  RecordPageControls,
  type RecordPageSize,
} from "../components/record_list/record_page_controls";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import { SearchLeaveGuard } from "../components/search_leave_guard";
import {
  SearchResultDisplay,
  type SearchResultDisplayMode,
} from "../components/search_result_display";
import { publicBlueprintContent } from "./blueprint_course_search_result";
import { RecordSortControl } from "../components/record_list/record_sort_control";
import { ApiProtocolError, ApiRequestError } from "../api/http_client";
import {
  BlueprintSearchClassification,
  emptyBlueprintClassificationSearch,
} from "./blueprint_course_search_classification";
import type { BlueprintSearchSnapshot as SearchSnapshot } from "./blueprint_course_search_return_state";
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
    tag: "",
  };
}

function hasConfiguredSearch(snapshot: SearchSnapshot): boolean {
  const classification = snapshot.classification;
  return (
    snapshot.query.trim().length > 0 ||
    snapshot.promotedOnly ||
    snapshot.tag.trim().length > 0 ||
    classification.disciplineUuid !== null ||
    classification.subjectUuid !== null ||
    classification.topicUuid !== null ||
    classification.subtopicUuid !== null ||
    classification.crossDiscipline
  );
}

function sortDescription(sort: BlueprintCourseListSort): string {
  switch (sort) {
    case "name":
      return "name";
    case "adoptions":
      return "most adoptions";
    case "students":
      return "most students having taken the course";
    case "stars":
      return "most stars";
    case "watches":
      return "most watches";
    case "recentEdits":
      return "most recently edited";
  }
}

function searchErrorMessage(failure: unknown): string {
  if (failure instanceof ApiProtocolError) return failure.message;
  if (failure instanceof ApiRequestError && failure.status === 401)
    return "Your session ended. Sign in again, then return to Public Blueprint Search.";
  if (failure instanceof ApiRequestError && failure.status === 403)
    return "Public Blueprint Search requires an active Instructor Account.";
  return "Public Blueprint Courses could not load. Try again.";
}

export function PublicBlueprintSearchResultList(props: {
  readonly courses: ReadonlyArray<BlueprintCourseSummaryView>;
  readonly state: RecordListState;
  readonly presentation: SearchResultDisplayMode;
}): JSX.Element {
  return (
    <RecordList
      ariaLabel="Available Public Blueprint Courses"
      emptyState={{
        title: "No available Public Blueprint Courses match.",
        message: "Try another name or classification, or clear the search.",
      }}
      recordId={(course) => course.id}
      content={publicBlueprintContent}
      presentation={props.presentation}
      rows={props.courses}
      state={props.state}
    />
  );
}

/** Local signals own filters and one server-returned page; the server owns public visibility. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  const sessionState = useSessionBootstrap().state();
  if (sessionState.kind !== "authenticated")
    throw new Error("Public Blueprint Search requires an authenticated session scope");
  const [draft, setDraft] = createSignal("");
  const [draftPromotedOnly, setDraftPromotedOnly] = createSignal(false);
  const [draftClassification, setDraftClassification] = createSignal(
    emptyBlueprintClassificationSearch(),
  );
  const [draftClassificationDescription, setDraftClassificationDescription] = createSignal("");
  const [draftTag, setDraftTag] = createSignal("");
  const [submitted, setSubmitted] = createSignal<SearchSnapshot>(emptySearch());
  const [courses, setCourses] = createSignal<ReadonlyArray<BlueprintCourseSummaryView>>([]);
  const [currentCursor, setCurrentCursor] = createSignal<string | undefined>();
  const [previousCursors, setPreviousCursors] = createSignal<ReadonlyArray<string | undefined>>([]);
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [pageSize, setPageSize] = createSignal<RecordPageSize>(50);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [pendingRequest, setPendingRequest] = createSignal<BlueprintPageRequest>();
  const [hasSearch, setHasSearch] = createSignal(false);
  const [presentation, setPresentation] = createSignal<SearchResultDisplayMode>("compact");
  let generation = 0;
  let resultsStatus: HTMLParagraphElement | undefined;

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
        target.snapshot.tag,
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
    setHasSearch(true);
    void load(
      pageRequest(
        {
          query: draft().trim(),
          promotedOnly: draftPromotedOnly(),
          classification: draftClassification(),
          classificationDescription: draftClassificationDescription(),
          sort: submitted().sort,
          tag: draftTag().trim(),
        },
        undefined,
        [],
        pageSize(),
        true,
      ),
    );
  }
  function resetSearch(): void {
    setHasSearch(true);
    setDraft("");
    setDraftPromotedOnly(false);
    setDraftClassification(emptyBlueprintClassificationSearch());
    setDraftClassificationDescription("");
    setDraftTag("");
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
  onCleanup(() => {
    generation += 1;
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
          <label class="blueprint-public-search__query" for="public-blueprint-tag">
            <span>Tag</span>
            <input
              id="public-blueprint-tag"
              type="search"
              value={draftTag()}
              onInput={(event) => setDraftTag(event.currentTarget.value)}
            />
          </label>
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
          {submitted().tag === "" ? "all tags" : `tag "${submitted().tag}"`};{" "}
          {sortDescription(submitted().sort)}.
        </p>
        <RecordSortControl
          label="Sort Public Blueprint Courses"
          options={[
            { value: "name", label: "Name" },
            { value: "adoptions", label: "Adoptions" },
            { value: "students", label: "Students having taken the course" },
            { value: "stars", label: "Stars" },
            { value: "watches", label: "Watches" },
            { value: "recentEdits", label: "Most recently edited" },
          ]}
          value={submitted().sort}
          disabled={loading()}
          onChange={changeSort}
        />
        <SearchResultDisplay
          ariaLabel="Blueprint result display"
          mode={presentation}
          onChange={setPresentation}
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
        <PublicBlueprintSearchResultList
          courses={courses()}
          state={collectionState()}
          presentation={presentation()}
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
      <SearchLeaveGuard
        hasSearch={() =>
          hasSearch() ||
          hasConfiguredSearch({
            ...submitted(),
            query: draft(),
            promotedOnly: draftPromotedOnly(),
            classification: draftClassification(),
            tag: draftTag(),
          })
        }
      />
    </PageFrame>
  );
}
