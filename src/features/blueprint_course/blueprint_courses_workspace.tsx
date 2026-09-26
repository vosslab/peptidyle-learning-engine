// Live discovery for reusable Blueprint Courses.
import { A } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseListSort } from "../../api/blueprint_course";
import { PageFrame } from "../../components/page_frame";
import {
  RecordPageControls,
  type RecordPageSize,
} from "../../components/record_list/record_page_controls";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../../components/record_list/record_list";
import { RecordSortControl } from "../../components/record_list/record_sort_control";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import { BlueprintCourseCreateDialog } from "./blueprint_course_create_dialog";
import { BlueprintCourseImport } from "./blueprint_exchange";
import type { BlueprintCoursesWorkspaceProps } from "./blueprint_course_workspace_types";
import "./blueprint_course.css";

type LoadState = "loading" | "ready" | "error";
type NoticeKind = "status" | "alert";

interface Notice {
  readonly kind: NoticeKind;
  readonly text: string;
}

interface BlueprintDiscoveryOptions {
  readonly includeArchived: boolean;
  readonly pageSize: RecordPageSize;
  readonly sort: BlueprintCourseListSort;
}

interface BlueprintPageRequest {
  readonly cursor: string | undefined;
  readonly previousCursors: ReadonlyArray<string | undefined>;
  readonly options: BlueprintDiscoveryOptions;
  readonly focusResults: boolean;
  readonly successNotice?: string;
}

function blueprintCoursePath(blueprintCourseId: string): string {
  return `/blueprint-courses/${encodeURIComponent(blueprintCourseId)}`;
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

function blueprintCourseContent(course: BlueprintCourseSummaryView): RecordContent {
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
      {
        kind: "text",
        label: "Access",
        value:
          course.read_access === "blueprint_course_owner"
            ? "You are the Blueprint Course Owner."
            : "Inspect its reusable modules.",
      },
      { kind: "courseClassification", value: course.classification },
    ],
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open Blueprint Course",
        href: blueprintCoursePath(course.id),
        primary: true,
      },
    ],
  };
}

export function BlueprintCoursesWorkspace(props: BlueprintCoursesWorkspaceProps): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [courses, setCourses] = createSignal<ReadonlyArray<BlueprintCourseSummaryView>>([]);
  const [currentCursor, setCurrentCursor] = createSignal<string>();
  const [previousCursors, setPreviousCursors] = createSignal<ReadonlyArray<string | undefined>>([]);
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);
  const [options, setOptions] = createSignal<BlueprintDiscoveryOptions>({
    includeArchived: false,
    pageSize: 50,
    sort: "name",
  });
  const [loading, setLoading] = createSignal(false);
  const [pendingRequest, setPendingRequest] = createSignal<BlueprintPageRequest>();
  const [notice, setNotice] = createSignal<Notice>({
    kind: "status",
    text: "Loading Blueprint Courses.",
  });
  let createTrigger: HTMLButtonElement | undefined;
  let resultsStatus: HTMLParagraphElement | undefined;
  let discoveryRequest = 0;
  function discoveryOptions(): BlueprintDiscoveryOptions {
    return options();
  }

  function pageRequest(
    cursor: string | undefined,
    previousCursors: ReadonlyArray<string | undefined>,
    options = discoveryOptions(),
    focusResults = false,
    successNotice?: string,
  ): BlueprintPageRequest {
    return { cursor, previousCursors, options, focusResults, successNotice };
  }

  function focusResults(): void {
    queueMicrotask(() => resultsStatus?.focus());
  }

  async function load(target: BlueprintPageRequest): Promise<void> {
    const request = ++discoveryRequest;
    const initialLoad = state() !== "ready";
    setPendingRequest(target);
    setLoading(true);
    if (initialLoad) setState("loading");
    try {
      const page = await props.client.listBlueprintCourses(
        target.cursor,
        target.options.pageSize,
        target.options.includeArchived ? true : undefined,
        undefined,
        false,
        false,
        undefined,
        target.options.sort,
      );
      if (request !== discoveryRequest) return;
      setCourses(page.items);
      setCurrentCursor(target.cursor);
      setPreviousCursors(target.previousCursors);
      setNextCursor(page.nextCursor);
      setOptions(target.options);
      setState("ready");
      setPendingRequest(undefined);
      setLoading(false);
      setNotice({
        kind: "status",
        text:
          target.successNotice ??
          "Choose a Blueprint Course to inspect or create a new reusable course structure.",
      });
      if (target.focusResults) focusResults();
    } catch (error: unknown) {
      if (request !== discoveryRequest) return;
      if (target.cursor !== undefined && error instanceof ApiRequestError && error.status === 400) {
        setPendingRequest(undefined);
        void load(
          pageRequest(
            undefined,
            [],
            target.options,
            true,
            "That Blueprint Course continuation is no longer available. Returned to the newest page.",
          ),
        );
        return;
      }
      setLoading(false);
      if (initialLoad) setState("error");
      else setState("ready");
      setNotice({
        kind: "alert",
        text: errorMessage(error, "Blueprint Courses could not load. Try again."),
      });
    }
  }

  function reset(options: BlueprintDiscoveryOptions): void {
    void load(pageRequest(undefined, [], options, true));
  }

  function changeIncludeArchived(next: boolean): void {
    reset({ ...discoveryOptions(), includeArchived: next });
  }

  function changeSort(next: BlueprintCourseListSort): void {
    if (next !== discoveryOptions().sort) reset({ ...discoveryOptions(), sort: next });
  }

  function changePageSize(next: RecordPageSize): void {
    if (next !== discoveryOptions().pageSize) reset({ ...discoveryOptions(), pageSize: next });
  }

  function collectionState(): RecordListState {
    if (state() === "loading" || loading()) {
      return { kind: "loading", label: "Loading Blueprint Courses." };
    }
    if (state() === "error" || pendingRequest() !== undefined) {
      return {
        kind: "error",
        title: "Blueprint Courses unavailable",
        message: notice().text,
        retry: (): void => {
          const target = pendingRequest();
          if (target !== undefined) void load(target);
        },
        retryLabel: "Retry loading Blueprint Courses",
      };
    }
    return { kind: "ready" };
  }

  function previousPage(): void {
    const cursors = previousCursors();
    if (cursors.length === 0) return;
    void load(pageRequest(cursors[cursors.length - 1], cursors.slice(0, -1), undefined, true));
  }

  function nextPage(): void {
    const cursor = nextCursor();
    if (cursor === null) return;
    void load(pageRequest(cursor, [...previousCursors(), currentCursor()], undefined, true));
  }

  onMount(() => void load(pageRequest(undefined, [])));

  return (
    <PageFrame
      eyebrow="Blueprint Courses"
      title="Build reusable course structure"
      lede="Blueprint Courses contain reusable modules and assessments, with no Students or delivery dates."
      routeSurface="blueprintCourses"
    >
      <Show when={props.proposalClient}>
        <A href="/blueprint-change-proposals">My Change Proposals</A>
      </Show>
      <Show when={state() === "ready" && !loading() && pendingRequest() === undefined}>
        <p
          class="blueprint-course-notice"
          role={notice().kind === "alert" ? "alert" : "status"}
          tabindex="-1"
          ref={(element) => {
            resultsStatus = element;
          }}
        >
          {notice().text}
        </p>
      </Show>
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
                checked={discoveryOptions().includeArchived}
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
            <BlueprintCourseImport client={props.client} />
          </div>
        </div>
        <RecordSortControl
          label="Sort Blueprint Courses"
          options={[
            { value: "name", label: "Name" },
            { value: "adoptions", label: "Total adoptions" },
            { value: "students", label: "Students ever enrolled" },
          ]}
          value={discoveryOptions().sort}
          disabled={state() !== "ready" || loading()}
          onChange={changeSort}
        />
        <RecordList
          ariaLabel="Available Blueprint Courses"
          emptyState={{
            title: "No Blueprint Courses are visible yet.",
            message: "Create the first one.",
          }}
          recordId={(course) => course.id}
          content={blueprintCourseContent}
          rows={courses()}
          state={collectionState()}
        />
        <RecordPageControls
          ariaLabel="Blueprint Course pages"
          hasPrevious={previousCursors().length > 0}
          hasNext={nextCursor() !== null}
          loading={loading()}
          disabled={state() !== "ready"}
          onPrevious={previousPage}
          onNext={nextPage}
          pageSize={discoveryOptions().pageSize}
          onPageSizeChange={changePageSize}
        />
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
    </PageFrame>
  );
}

/** Loads one opaque Blueprint Course and lets its owner Save complete Revision content. */
