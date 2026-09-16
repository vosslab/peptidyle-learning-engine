// course_list_page.tsx - live Course Instance creation, empty or from exact Blueprint Revisions.

import { A, useSearchParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { CourseInstanceCreationSource, CourseInstanceSummary } from "../api/course_instance";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { courseThemeTokens } from "../features/course_appearance/course_theme_registry";
import {
  courseInstanceRouteReference,
  parseBlueprintCourseReference,
} from "../navigation/public_route";
import { StudentCoursesPage } from "./student_courses_page";

type AdoptableBlueprintCourse = BlueprintCourseSummaryView;

function isAdoptableBlueprintCourse(
  blueprint: BlueprintCourseSummaryView,
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

/** Course Instance list and Instructor creation task. */
function TeachingCourseListPage(): JSX.Element {
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
  const [blueprints] = createResource(
    () => isInstructor() && creationSource() === "adopted",
    async (adopting) => (adopting ? applicationApi.client.listBlueprintCourses() : { items: [] }),
  );
  const [createdCourses, setCreatedCourses] = createSignal<ReadonlyArray<CourseInstanceSummary>>(
    [],
  );
  const [blueprintChoice, setBlueprintChoice] = createSignal<string>();
  const blueprintSource = (): string => {
    const choice = blueprintChoice();
    if (choice !== undefined) return choice;
    const selected = adoptableBlueprints().find(
      (blueprint) => blueprint.reference === searchParams.blueprint,
    );
    return selected === undefined ? "" : blueprintSourceValue(selected);
  };
  const [shortName, setShortName] = createSignal("");
  const [longName, setLongName] = createSignal("");
  const [startDate, setStartDate] = createSignal("");
  const [endDate, setEndDate] = createSignal("");
  const [isCreating, setIsCreating] = createSignal(false);
  const [creationError, setCreationError] = createSignal<string | null>(null);

  const visibleCourses = createMemo(() => {
    const seen = new Set<string>();
    return [...createdCourses(), ...(courses() ?? [])].filter((course) => {
      if (seen.has(course.reference)) return false;
      seen.add(course.reference);
      return true;
    });
  });
  const adoptableBlueprints = createMemo(() =>
    blueprints.error === undefined
      ? (blueprints()?.items ?? []).filter(isAdoptableBlueprintCourse)
      : [],
  );

  async function createCourseInstance(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (isCreating()) return;
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
        source,
        shortName: shortName(),
        longName: longName(),
        term: { startDate: startDate(), endDate: endDate() },
      });
      setCreatedCourses((current) => [created.course, ...current]);
      setCreationSource("empty");
      setBlueprintChoice("");
      setShortName("");
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
    <section class="page" data-route-surface="courses">
      <p class="eyebrow">Teaching</p>
      <h1>{isInstructor() ? "Course Instances you teach" : "Your Course Instances"}</h1>
      <p class="page-lede">
        Start an empty Course Instance or adopt a Blueprint Course with its Assessments. Review
        dates and settings before releasing Assessments to students.
      </p>
      <Show when={isInstructor()}>
        <form
          id="create-course-instance"
          class="course-create-form"
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
              </p>
            </Show>
            <Show
              when={adoptableBlueprints().length > 0}
              fallback={
                <Show when={!blueprints.loading && blueprints.error === undefined}>
                  <p class="empty-state">No public Blueprint Courses are available to adopt.</p>
                </Show>
              }
            >
              <BlueprintSourceSelect
                blueprints={adoptableBlueprints()}
                value={blueprintSource()}
                onChange={setBlueprintChoice}
              />
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
            <p class="empty-state">No Course Instances are teaching yet.</p>
          </Show>
        }
      >
        <div class="instructor-list" aria-label="Course Instances">
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
    <Show when={isStudent()} fallback={<TeachingCourseListPage />}>
      <StudentCoursesPage />
    </Show>
  );
}
