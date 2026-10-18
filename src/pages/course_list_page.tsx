// course_list_page.tsx - live Course Instance creation, empty or from exact Blueprint Revisions.

import { useSearchParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, Show, type JSX } from "solid-js";

import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type {
  CourseInstanceCreationSource,
  CourseInstanceLifecycleState,
  CourseInstanceSummary,
} from "../api/course_instance";
import type { BlueprintCourseClient } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { courseInstanceRouteId, parseBlueprintCourseId } from "../navigation/public_route";
import { StudentCoursesPage } from "./student_courses_page";
import {
  CourseClassificationFields,
  emptyCourseClassification,
  type CourseClassificationDraft,
} from "../components/course_classification_fields";
import { PageFrame } from "../components/page_frame/page_frame";
import type { RecordContent } from "../components/data_sheet/record_content";
import { decodeCourseClassification } from "../api/decoders/course_classification";
import { courseTermFitsActiveLifetime } from "../api/decoders/course_term";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createSearchState } from "../components/search_block/search_state";
import { useSelectedDisplayDateTimeFormatter } from "../selected_display_zone";
import {
  blueprintCourseSearchDefinition,
  emptyBlueprintSearch,
} from "./blueprint_course_search_definition";
import {
  blueprintCourseSearchFields,
  blueprintCourseSearchRecordQuery,
} from "./blueprint_course_search_fields";
import { courseCollectionFields } from "./course_collection_fields";
import "./course_list_page.css";

type AdoptableBlueprintCourse = Pick<
  BlueprintCourseSummaryView,
  "id" | "long_name" | "availability" | "current_revision_tuple"
>;

function isAdoptableBlueprintCourse(
  blueprint: AdoptableBlueprintCourse,
): blueprint is AdoptableBlueprintCourse {
  return blueprint.availability === "public";
}

function blueprintSourceValue(blueprint: AdoptableBlueprintCourse): string {
  const revision = blueprint.current_revision_tuple;
  return `${revision.blueprintCourseId}:${revision.revisionNumber}`;
}

type CourseListMode = CourseInstanceLifecycleState;

function BlueprintSourcePicker(props: {
  readonly client: BlueprintCourseClient & ContentClassificationClient;
  readonly value: string;
  readonly onChoose: (blueprint: BlueprintCourseSummaryView) => void;
}): JSX.Element {
  const formatTimestamp = useSelectedDisplayDateTimeFormatter();
  const definition = blueprintCourseSearchDefinition(props.client, formatTimestamp, "public");
  const state = createSearchState(
    {
      ...definition,
      initialQuery: emptyBlueprintSearch(false),
      isEligible: () => true,
      content: (blueprint: BlueprintCourseSummaryView): RecordContent => ({
        ...definition.content(blueprint),
        actions: [
          {
            id: "choose-source",
            kind: "command",
            label: "Choose Blueprint",
            primary: true,
            pressed: props.value === blueprintSourceValue(blueprint),
            onClick: () => props.onChoose(blueprint),
          },
        ],
      }),
    },
    { runOnMount: true },
  );
  return (
    <SearchBlock
      layout="above"
      state={state}
      controls={<SearchControls state={state} textLabel="Search public Blueprint sources" />}
      collection={{
        ariaLabel: "Public Blueprint sources",
        fields: blueprintCourseSearchFields(props.client, "public", formatTimestamp),
        query: blueprintCourseSearchRecordQuery({
          query: state.query,
          apply: state.apply,
          sort: state.sort,
          client: props.client,
          scope: "public",
        }),
        emptyState: {
          title: "No public Blueprint sources match.",
          message: "Try another name or clear the search.",
        },
      }}
    />
  );
}

/** Course Instance list and, for active Courses, Instructor creation task. */
function TeachingCourseListPage(props: { readonly mode: CourseListMode }): JSX.Element {
  const applicationApi = useApplicationApi();
  const [searchParams] = useSearchParams();
  const session = useSessionBootstrap();
  const isInstructor = createMemo(() => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "instructor";
  });
  const [courses, { refetch: refetchCourses, mutate: mutateCourses }] = createResource(
    isInstructor,
    async (instructor) => (instructor ? applicationApi.client.listCourseInstances() : []),
  );
  const [creationSource, setCreationSource] = createSignal<"empty" | "adopted">(
    typeof searchParams.blueprint === "string" &&
      parseBlueprintCourseId(searchParams.blueprint) !== null
      ? "adopted"
      : "empty",
  );
  const [linkedBlueprint, { refetch: refetchLinkedBlueprint }] = createResource(
    () => {
      if (!isInstructor() || creationSource() !== "adopted") return false;
      // ASVS 2.2.1: only canonical Blueprint Course IDs enter the exact-source request.
      return typeof searchParams.blueprint === "string"
        ? (parseBlueprintCourseId(searchParams.blueprint) ?? false)
        : false;
    },
    async (blueprintCourseId) =>
      (await applicationApi.client.getBlueprintCourse(blueprintCourseId)).blueprintCourse,
  );
  const [blueprintChoice, setBlueprintChoice] = createSignal<string>();
  const [chosenBlueprint, setChosenBlueprint] = createSignal<AdoptableBlueprintCourse>();
  const blueprintSource = (): string => {
    const choice = blueprintChoice();
    if (choice !== undefined) return choice;
    const selected = linkedBlueprint.error === undefined ? linkedBlueprint() : undefined;
    return selected === undefined || !isAdoptableBlueprintCourse(selected)
      ? ""
      : blueprintSourceValue(selected);
  };
  const [shortName, setShortName] = createSignal("");
  const [classification, setClassification] = createSignal<CourseClassificationDraft>(
    emptyCourseClassification(),
  );
  const [longName, setLongName] = createSignal("");
  const [startDate, setStartDate] = createSignal("");
  const [endDate, setEndDate] = createSignal("");
  const [isCreating, setIsCreating] = createSignal(false);
  const [creationError, setCreationError] = createSignal<string | null>(null);
  const [createdCourseFocusId, setCreatedCourseFocusId] = createSignal<string>();
  const [creationDisclosure, setCreationDisclosure] = createSignal<boolean | undefined>(
    creationSource() === "adopted" ? true : undefined,
  );

  const courseContent = (course: CourseInstanceSummary): RecordContent => ({
    title: course.longName,
    details: [
      { kind: "courseClassification", value: course.classification },
      {
        kind: "text",
        label: "Term",
        value: `${course.term.startDate} through ${course.term.endDate}`,
      },
    ],
    actions: [
      {
        id: "open-course",
        kind: "link",
        label: "Open Course",
        href: `/courses/${courseInstanceRouteId(course.id)}`,
        primary: true,
        ref: (element): void => {
          if (createdCourseFocusId() !== course.id) return;
          queueMicrotask(() => {
            if (createdCourseFocusId() === course.id) {
              element.focus();
              setCreatedCourseFocusId(undefined);
            }
          });
        },
      },
    ],
  });

  const courseSearch = createLocalCompleteSearch(
    {
      rows: courses,
      loading: () => courses.loading,
      error: (): unknown => courses.error,
      refetchSource: refetchCourses,
    },
    {
      fields: courseCollectionFields,
      rowId: (course) => course.id,
      keyword: (course) =>
        `${course.longName} ${course.shortName} ${course.term.startDate} ${course.term.endDate} ${course.classification.tags.join(" ")}`,
      inScope: (course) => course.lifecycleState === props.mode,
      content: courseContent,
    },
  );
  const isActiveMode = (): boolean => props.mode === "active";
  const isCreationExpanded = createMemo(
    () =>
      creationDisclosure() ??
      (isActiveMode() &&
        !courses.loading &&
        courses.error === undefined &&
        (courses()?.filter((course) => course.lifecycleState === props.mode).length ?? 0) === 0),
  );
  const adoptableBlueprints = createMemo(() => {
    const linked = linkedBlueprint.error === undefined ? linkedBlueprint() : undefined;
    const chosen = chosenBlueprint();
    const seen = new Set<string>();
    const candidates: AdoptableBlueprintCourse[] = linked ? [linked] : [];
    if (chosen) {
      const linkedIndex = candidates.findIndex((blueprint) => blueprint.id === chosen.id);
      if (linkedIndex === -1) candidates.push(chosen);
      else candidates[linkedIndex] = chosen;
    }
    return candidates.filter((blueprint) => {
      if (!isAdoptableBlueprintCourse(blueprint) || seen.has(blueprint.id)) return false;
      seen.add(blueprint.id);
      return true;
    });
  });

  async function createCourseInstance(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (isCreating()) return;
    let selectedClassification;
    try {
      selectedClassification = decodeCourseClassification(classification());
    } catch {
      setCreationError(
        "Choose a Discipline and check that Tags are unique, trimmed labels of 1 through 120 characters.",
      );
      return;
    }
    let source: CourseInstanceCreationSource = { kind: "empty" };
    if (creationSource() === "adopted") {
      const selected = adoptableBlueprints().find(
        (blueprint) => blueprintSourceValue(blueprint) === blueprintSource(),
      );
      if (selected === undefined) {
        setCreationError("Choose the current Blueprint Course Revision for this Course Instance.");
        return;
      }
      source = {
        kind: "adopted",
        blueprintCourseRevisionTuple: selected.current_revision_tuple,
      };
    }
    if (
      shortName().trim() !== shortName() ||
      shortName().trim().length === 0 ||
      longName().trim() !== longName() ||
      longName().trim().length === 0
    ) {
      setCreationError("Enter trimmed Course short and long names.");
      return;
    }
    if (startDate() === "" || endDate() === "" || endDate() < startDate()) {
      setCreationError("Enter an end date on or after the Course Term start date.");
      return;
    }
    if (!courseTermFitsActiveLifetime(endDate(), new Date().toISOString().slice(0, 10))) {
      setCreationError(
        "Choose an end date within six months of today. A Course remains Active for at most six months from creation.",
      );
      return;
    }
    setCreationError(null);
    setIsCreating(true);
    try {
      const created = await applicationApi.client.createCourseInstance({
        classification: selectedClassification,
        source,
        shortName: shortName(),
        longName: longName(),
        term: { startDate: startDate(), endDate: endDate() },
      });
      if (created.courseInstance.lifecycleState !== "active") {
        throw new Error("A newly created Course Instance must be Active.");
      }
      await courseSearch.state.clear();
      setCreatedCourseFocusId(created.courseInstance.id);
      mutateCourses((current) => [created.courseInstance, ...(current ?? [])]);
      setCreationDisclosure(false);
      setCreationSource("empty");
      setBlueprintChoice("");
      setChosenBlueprint(undefined);
      setShortName("");
      setClassification(emptyCourseClassification());
      setLongName("");
      setStartDate("");
      setEndDate("");
      void refetchCourses();
    } catch (_error: unknown) {
      setCreationError("We could not create that Course Instance. Check the source and try again.");
    } finally {
      setIsCreating(false);
    }
  }

  return (
    <PageFrame
      routeSurface={isActiveMode() ? "courses" : "inactiveCourses"}
      actions={
        isInstructor() && isActiveMode() ? (
          <button
            class="quiet-action course-create-disclosure"
            type="button"
            aria-expanded={isCreationExpanded()}
            aria-controls="create-course-instance"
            disabled={isCreating()}
            onClick={() => setCreationDisclosure(!isCreationExpanded())}
          >
            <span aria-hidden="true">{isCreationExpanded() ? "\u25be" : "\u25b8"}</span>
            Create Course Instance
          </button>
        ) : undefined
      }
    >
      <Show when={isInstructor() && isActiveMode()}>
        <div id="create-course-instance" hidden={!isCreationExpanded()}>
          <label for="course-creation-source">
            Start with
            <select
              id="course-creation-source"
              name="creationSource"
              form="create-course-instance-form"
              value={creationSource()}
              onInput={(event) => {
                setCreationSource(event.currentTarget.value === "adopted" ? "adopted" : "empty");
                setCreationError(null);
              }}
            >
              <option value="empty">An empty Course Instance</option>
              <option value="adopted">A Blueprint Course Revision</option>
            </select>
          </label>
          <Show when={creationSource() === "adopted"}>
            <Show when={linkedBlueprint.loading}>
              <p class="loading-state">Loading the linked Blueprint Course...</p>
            </Show>
            <Show when={linkedBlueprint.error !== undefined}>
              <p class="route-error" role="alert">
                The linked Blueprint Course could not be loaded. Choose a Public Blueprint Course or
                try again.
                <button type="button" onClick={() => void refetchLinkedBlueprint()}>
                  Try again
                </button>
              </p>
            </Show>
            <Show
              when={
                linkedBlueprint.error === undefined &&
                linkedBlueprint() !== undefined &&
                !isAdoptableBlueprintCourse(linkedBlueprint()!)
              }
            >
              <p class="route-error" role="alert">
                Only Public Blueprint Courses can be adopted. Choose a Public Blueprint Course.
              </p>
            </Show>
            <BlueprintSourcePicker
              client={applicationApi.client}
              value={blueprintSource()}
              onChoose={(blueprint) => {
                setBlueprintChoice(blueprintSourceValue(blueprint));
                setChosenBlueprint(blueprint);
              }}
            />
            <Show
              when={adoptableBlueprints().find(
                (blueprint) => blueprintSourceValue(blueprint) === blueprintSource(),
              )}
            >
              {(blueprint) => (
                <p role="status">
                  Selected source: {blueprint().long_name} · Revision{" "}
                  {blueprint().current_revision_tuple.revisionNumber}
                </p>
              )}
            </Show>
          </Show>
          <form
            id="create-course-instance-form"
            class="course-create-form"
            aria-busy={isCreating()}
            novalidate
            onSubmit={(event) => void createCourseInstance(event)}
          >
            <Show when={creationSource() === "adopted"}>
              <input type="hidden" name="blueprintSource" value={blueprintSource()} />
            </Show>
            <label for="course-short-name">
              Course short name
              <input
                id="course-short-name"
                name="shortName"
                type="text"
                value={shortName()}
                onInput={(event) => setShortName(event.currentTarget.value)}
                autocomplete="off"
                required
              />
              <small>For compact navigation; about 16 characters when practical.</small>
            </label>
            <label for="course-long-name">
              Course long name
              <input
                id="course-long-name"
                name="longName"
                type="text"
                value={longName()}
                onInput={(event) => setLongName(event.currentTarget.value)}
                autocomplete="off"
                required
              />
            </label>
            <CourseClassificationFields
              value={classification()}
              disabled={isCreating()}
              onChange={setClassification}
            />
            <label for="course-start-date">
              Course Term start date
              <input
                id="course-start-date"
                name="startDate"
                type="date"
                value={startDate()}
                onInput={(event) => setStartDate(event.currentTarget.value)}
                required
              />
            </label>
            <label for="course-end-date">
              Course Term end date
              <input
                id="course-end-date"
                name="endDate"
                type="date"
                value={endDate()}
                onInput={(event) => setEndDate(event.currentTarget.value)}
                required
              />
            </label>
            <button class="primary-action" type="submit" disabled={isCreating()}>
              {isCreating() ? "Creating Course Instance..." : "Create Course Instance"}
            </button>
            <Show when={creationError()}>{(error) => <p role="alert">{error()}</p>}</Show>
          </form>
        </div>
      </Show>
      <Show when={!isInstructor()}>
        <p class="empty-state">Course access begins when you hold an active Course Membership.</p>
      </Show>
      <Show when={isInstructor()}>
        <SearchBlock
          layout="above"
          controls={
            <SearchControls
              state={courseSearch.state}
              textLabel={isActiveMode() ? "Search active Courses" : "Search inactive Courses"}
            />
          }
          state={courseSearch.state}
          collection={{
            ariaLabel: isActiveMode() ? "Active Course Instances" : "Inactive Course Instances",
            fields: courseCollectionFields,
            query: courseSearch.query,
            get emptyState() {
              return {
                title:
                  courseSearch.state.query().text || courseSearch.state.query().filters.size > 0
                    ? "No Course Instances match these filters"
                    : isActiveMode()
                      ? "No Course Instances are teaching yet"
                      : "No inactive Courses",
                message: isActiveMode()
                  ? undefined
                  : "Inactive Course Instances appear here after their Course Term ends.",
              };
            },
          }}
        />
      </Show>
    </PageFrame>
  );
}

/** Canonical Course index for the signed-in Account's User Role. */
export function CourseListPage(): JSX.Element {
  const session = useSessionBootstrap();
  const isStudent = (): boolean => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "student";
  };
  return (
    <Show when={isStudent()} fallback={<TeachingCourseListPage mode="active" />}>
      <StudentCoursesPage />
    </Show>
  );
}

/** Instructor-only past Course Instance list. */
export function InactiveCourseListPage(): JSX.Element {
  return <TeachingCourseListPage mode="inactive" />;
}
