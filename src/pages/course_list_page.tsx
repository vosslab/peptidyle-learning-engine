// course_list_page.tsx - live Course Instance creation, empty or from exact Blueprint Revisions.

import { A, useSearchParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type {
  CourseInstanceCreationSource,
  CourseInstanceLifecycleState,
  CourseInstanceSummary,
} from "../api/course_instance";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { courseThemeTokens } from "../features/course_appearance/course_theme_registry";
import {
  courseInstanceRouteReference,
  parseBlueprintCourseReference,
} from "../navigation/public_route";
import { StudentCoursesPage } from "./student_courses_page";
import {
  CourseClassificationFields,
  emptyCourseClassification,
  type CourseClassificationDraft,
} from "../components/course_classification_fields";
import { CourseClassificationSummary } from "../components/course_classification_summary";
import { decodeCourseClassification } from "../api/decoders/course_classification";
import "./course_list_page.css";

type AdoptableBlueprintCourse = Pick<
  BlueprintCourseSummaryView,
  "reference" | "long_name" | "availability" | "current_revision"
>;

function isAdoptableBlueprintCourse(
  blueprint: AdoptableBlueprintCourse,
): blueprint is AdoptableBlueprintCourse {
  return blueprint.availability === "public";
}

function blueprintSourceValue(blueprint: AdoptableBlueprintCourse): string {
  const revision = blueprint.current_revision;
  return `${revision.reference}:${revision.revision}`;
}

function CourseInstanceRow(props: { readonly course: CourseInstanceSummary }): JSX.Element {
  const reference = courseInstanceRouteReference(props.course.reference);
  const theme = courseThemeTokens(props.course.theme);
  return (
    <article
      class="instructor-list__row instructor-list__row--course"
      style={`--ple-instructor-list-theme-accent: ${theme.anchors.accent}`}
    >
      <div class="instructor-list__identity">
        <p class="instructor-list__kind">Course Instance</p>
        <h2>{props.course.longName}</h2>
        <CourseClassificationSummary value={props.course.classification} />
        <p class="instructor-list__metadata">
          {props.course.term.startDate} through {props.course.term.endDate}
        </p>
      </div>
      <p class="instructor-list__theme" aria-label={`Course theme: ${theme.name}`}>
        Theme: {theme.name}
      </p>
      <div class="instructor-list__actions">
        <A class="primary-link" href={`/courses/${reference}`} id={`course-open-${reference}`}>
          Open Course Instance
        </A>
      </div>
    </article>
  );
}

type CourseListMode = CourseInstanceLifecycleState;

function coursesForMode(
  createdCourses: ReadonlyArray<CourseInstanceSummary>,
  listedCourses: ReadonlyArray<CourseInstanceSummary> | undefined,
  mode: CourseListMode,
): ReadonlyArray<CourseInstanceSummary> {
  const listedReferences = new Set(listedCourses?.map((course) => course.reference) ?? []);
  const localOnlyCourses = createdCourses.filter(
    (course) => !listedReferences.has(course.reference),
  );
  const combinedCourses = [...localOnlyCourses, ...(listedCourses ?? [])];
  return combinedCourses.filter((course) => course.lifecycleState === mode);
}

function BlueprintSourceSelect(props: {
  readonly blueprints: ReadonlyArray<AdoptableBlueprintCourse>;
  readonly value: string;
  readonly onChange: (value: string) => void;
}): JSX.Element {
  return (
    <label for="course-blueprint-source">
      Blueprint Course
      <select
        id="course-blueprint-source"
        name="blueprintSource"
        value={props.value}
        onInput={(event) => props.onChange(event.currentTarget.value)}
        required
      >
        <option value="">Choose a reusable Blueprint Course</option>
        <For each={props.blueprints}>
          {(blueprint) => (
            <option value={blueprintSourceValue(blueprint)}>
              {blueprint.long_name} · Revision {blueprint.current_revision.revision}
            </option>
          )}
        </For>
      </select>
    </label>
  );
}

/** Course Instance list and, for active Courses, Instructor creation task. */
function TeachingCourseListPage(props: { readonly mode: CourseListMode }): JSX.Element {
  const applicationApi = useApplicationApi();
  const [searchParams] = useSearchParams();
  const session = useSessionBootstrap();
  const isInstructor = createMemo(() => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.productRole === "instructor";
  });
  const [courses, { refetch: refetchCourses }] = createResource(isInstructor, async (instructor) =>
    instructor ? applicationApi.client.listCourseInstances() : [],
  );
  const [creationSource, setCreationSource] = createSignal<"empty" | "adopted">(
    typeof searchParams.blueprint === "string" &&
      parseBlueprintCourseReference(searchParams.blueprint) !== null
      ? "adopted"
      : "empty",
  );
  const [blueprintCursors, setBlueprintCursors] = createSignal<ReadonlyArray<string>>([""]);
  const [blueprints, { refetch: refetchBlueprints }] = createResource(
    () =>
      isInstructor() &&
      creationSource() === "adopted" &&
      blueprintCursors()[blueprintCursors().length - 1],
    async (cursor) =>
      applicationApi.client.listBlueprintCourses(cursor || undefined, 50, false, undefined, true),
  );
  const [linkedBlueprint, { refetch: refetchLinkedBlueprint }] = createResource(
    () => {
      if (!isInstructor() || creationSource() !== "adopted") return false;
      // ASVS 2.2.1: only canonical Blueprint references enter the exact-source request.
      return typeof searchParams.blueprint === "string"
        ? (parseBlueprintCourseReference(searchParams.blueprint) ?? false)
        : false;
    },
    async (reference) =>
      (await applicationApi.client.getBlueprintCourse(reference)).blueprintCourse,
  );
  const [createdCourses, setCreatedCourses] = createSignal<ReadonlyArray<CourseInstanceSummary>>(
    [],
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
  const [creationDisclosure, setCreationDisclosure] = createSignal<boolean | undefined>(
    creationSource() === "adopted" ? true : undefined,
  );

  const visibleCourses = createMemo(() =>
    coursesForMode(
      createdCourses(),
      courses.error === undefined ? courses() : undefined,
      props.mode,
    ),
  );
  const isActiveMode = (): boolean => props.mode === "active";
  const isCreationExpanded = createMemo(
    () =>
      creationDisclosure() ??
      (isActiveMode() &&
        !courses.loading &&
        courses.error === undefined &&
        visibleCourses().length === 0),
  );
  const adoptableBlueprints = createMemo(() => {
    const linked = linkedBlueprint.error === undefined ? linkedBlueprint() : undefined;
    const chosen = chosenBlueprint();
    const page = blueprints.error === undefined ? (blueprints()?.items ?? []) : [];
    const seen = new Set<string>();
    return [...(linked ? [linked] : []), ...(chosen ? [chosen] : []), ...page].filter(
      (blueprint) => {
        if (!isAdoptableBlueprintCourse(blueprint) || seen.has(blueprint.reference)) return false;
        seen.add(blueprint.reference);
        return true;
      },
    );
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
        blueprintCourse: selected.reference,
        blueprintRevision: selected.current_revision.revision,
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
      if (created.course.lifecycleState !== "active") {
        throw new Error("A newly created Course Instance must be Active.");
      }
      setCreatedCourses((current) => [created.course, ...current]);
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
      queueMicrotask(() =>
        document
          .getElementById(`course-open-${courseInstanceRouteReference(created.course.reference)}`)
          ?.focus(),
      );
    } catch (_error: unknown) {
      setCreationError("We could not create that Course Instance. Check the source and try again.");
    } finally {
      setIsCreating(false);
    }
  }

  return (
    <section class="page" data-route-surface={isActiveMode() ? "courses" : "inactiveCourses"}>
      <p class="eyebrow">Teaching</p>
      <h1>{isActiveMode() ? "My Active Courses" : "My Inactive Courses"}</h1>
      <p class="page-lede">
        {isActiveMode()
          ? "Start an empty Course Instance or adopt a Blueprint Course with its Assessments. Review dates and settings before releasing Assessments to students."
          : "Past Course Instances stay available here without competing with the Courses you are currently teaching."}
      </p>
      <Show when={isInstructor() && isActiveMode()}>
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
        <form
          id="create-course-instance"
          class="course-create-form"
          hidden={!isCreationExpanded()}
          aria-busy={isCreating()}
          novalidate
          onSubmit={(event) => void createCourseInstance(event)}
        >
          <h2>Create Course Instance</h2>
          <label for="course-creation-source">
            Start with
            <select
              id="course-creation-source"
              name="creationSource"
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
            <Show when={blueprints.loading}>
              <p class="loading-state">Loading Blueprint Courses...</p>
            </Show>
            <Show when={blueprints.error !== undefined}>
              <p class="route-error" role="alert">
                Blueprint Courses could not be loaded.
                <button type="button" onClick={() => void refetchBlueprints()}>
                  Try again
                </button>
              </p>
            </Show>
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
            <Show
              when={adoptableBlueprints().length > 0}
              fallback={
                <Show
                  when={
                    !blueprints.loading &&
                    !linkedBlueprint.loading &&
                    blueprints.error === undefined
                  }
                >
                  <p class="empty-state">No public Blueprint Courses are available to adopt.</p>
                </Show>
              }
            >
              <BlueprintSourceSelect
                blueprints={adoptableBlueprints()}
                value={blueprintSource()}
                onChange={(value) => {
                  setBlueprintChoice(value);
                  setChosenBlueprint(
                    adoptableBlueprints().find(
                      (blueprint) => blueprintSourceValue(blueprint) === value,
                    ),
                  );
                }}
              />
            </Show>
            <Show when={blueprintCursors().length > 1}>
              <button
                type="button"
                disabled={blueprints.loading}
                onClick={() => setBlueprintCursors((cursors) => cursors.slice(0, -1))}
              >
                Previous Blueprint Courses
              </button>
            </Show>
            <Show when={blueprints.error === undefined && blueprints()?.nextCursor}>
              {(cursor) => (
                <button
                  type="button"
                  disabled={blueprints.loading}
                  onClick={() => setBlueprintCursors((cursors) => [...cursors, cursor()])}
                >
                  Next Blueprint Courses
                </button>
              )}
            </Show>
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
          <p role="status" aria-live="polite" aria-atomic="true">
            {creationError() ?? ""}
          </p>
        </form>
      </Show>
      <Show when={!isInstructor()}>
        <p class="empty-state">Course access begins when you hold an active Course Membership.</p>
      </Show>
      <Show when={courses.loading}>
        <p class="loading-state">Loading Course Instances...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <p>Course Instances could not be loaded.</p>
          <button class="primary-action" type="button" onClick={() => void refetchCourses()}>
            Try again
          </button>
        </section>
      </Show>
      <Show
        when={visibleCourses().length > 0}
        fallback={
          <Show when={!courses.loading && courses.error === undefined && isInstructor()}>
            <p class="empty-state">
              {isActiveMode() ? (
                <>
                  No Course Instances are teaching yet. Use Create Course Instance to start an empty
                  Course or adopt a Blueprint Course.
                </>
              ) : (
                <>
                  No past Course Instances are available. Open{" "}
                  <A href="/instructor">My Active Courses</A> to create a Course Instance.
                </>
              )}
            </p>
          </Show>
        }
      >
        <div
          class="instructor-list"
          aria-label={isActiveMode() ? "Active Course Instances" : "Inactive Course Instances"}
        >
          <For each={visibleCourses()}>{(course) => <CourseInstanceRow course={course} />}</For>
        </div>
      </Show>
    </section>
  );
}

/** Canonical Course index for the signed-in Account's Product Role. */
export function CourseListPage(): JSX.Element {
  const session = useSessionBootstrap();
  const isStudent = (): boolean => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.productRole === "student";
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
