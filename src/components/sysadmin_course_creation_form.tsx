// Sysadmin home form. The selected Instructor teaches the Course.

import { createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { CourseCreationInstructor } from "../api/course_instance";
import { useApplicationApi } from "../api/application_api";
import {
  CourseClassificationFields,
  emptyCourseClassification,
} from "./course_classification_fields";
import {
  CourseCreationInputError,
  submitSysadminCourseCreation,
  type SysadminCourseCreationDraft,
} from "./sysadmin_course_creation";

const unavailableInstructors: ReadonlyArray<CourseCreationInstructor> = [];

/** Loads Instructor Account IDs and creates a Course those Instructors teach. */
export function SysadminCourseCreation(): JSX.Element {
  const api = useApplicationApi();
  const [instructors, { refetch }] = createResource(async () =>
    api.client.listCourseCreationInstructors(),
  );
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal("");

  async function create(draft: SysadminCourseCreationDraft): Promise<void> {
    if (creating()) return;
    setCreating(true);
    setError(null);
    try {
      const message = await submitSysadminCourseCreation(
        api.client,
        draft,
        new Date().toISOString().slice(0, 10),
      );
      setStatus(message);
    } catch (caught) {
      setStatus("");
      setError(
        caught instanceof CourseCreationInputError
          ? caught.message
          : "We could not create that Course Instance. Check the Instructor and the Course details, then try again.",
      );
    } finally {
      setCreating(false);
    }
  }

  return (
    <Show
      when={instructors.error === undefined}
      fallback={
        <p class="route-error" role="alert">
          Instructor Accounts could not be loaded.
          <button type="button" onClick={() => void refetch()}>
            Try again
          </button>
        </p>
      }
    >
      <SysadminCourseCreationForm
        instructors={instructors() ?? unavailableInstructors}
        loading={instructors.loading}
        disabled={creating()}
        error={error()}
        status={status()}
        onSubmit={create}
      />
    </Show>
  );
}

export function SysadminCourseCreationForm(props: {
  readonly instructors: ReadonlyArray<CourseCreationInstructor>;
  readonly loading?: boolean;
  readonly disabled?: boolean;
  readonly error?: string | null;
  readonly status?: string;
  readonly onSubmit: (draft: SysadminCourseCreationDraft) => Promise<void>;
}): JSX.Element {
  const [assignedInstructorAccountId, setAssignedInstructorAccountId] = createSignal("");
  const [classification, setClassification] = createSignal(emptyCourseClassification());
  const [shortName, setShortName] = createSignal("");
  const [longName, setLongName] = createSignal("");
  const [startDate, setStartDate] = createSignal("");
  const [endDate, setEndDate] = createSignal("");
  const noInstructors = (): boolean => !props.loading && props.instructors.length === 0;

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    await props.onSubmit({
      assignedInstructorAccountId: assignedInstructorAccountId(),
      classification: classification(),
      shortName: shortName(),
      longName: longName(),
      startDate: startDate(),
      endDate: endDate(),
    });
    if (props.error === null || props.error === undefined) {
      setAssignedInstructorAccountId("");
      setClassification(emptyCourseClassification());
      setShortName("");
      setLongName("");
      setStartDate("");
      setEndDate("");
    }
  }

  return (
    <form
      id="sysadmin-create-course"
      class="course-create-form"
      aria-busy={props.disabled}
      novalidate
      onSubmit={(event) => void submit(event)}
    >
      <h2>Create Course Instance</h2>
      <p>
        The selected Instructor teaches this Course. Creating it does not add you as an Instructor.
      </p>
      <label for="sysadmin-course-instructor">
        Instructor Account ID
        <select
          id="sysadmin-course-instructor"
          name="assignedInstructorAccountId"
          value={assignedInstructorAccountId()}
          required
          disabled={props.disabled || props.loading || noInstructors()}
          onInput={(event) => setAssignedInstructorAccountId(event.currentTarget.value)}
        >
          <option value="">
            {props.loading ? "Loading Instructor Accounts..." : "Select Instructor Account ID"}
          </option>
          <For each={props.instructors}>
            {(instructor) => <option value={instructor.accountId}>{instructor.accountId}</option>}
          </For>
        </select>
      </label>
      <Show when={noInstructors()}>
        <p role="status">No Instructor Accounts are available.</p>
      </Show>
      <label for="sysadmin-course-short-name">
        Course short name
        <input
          id="sysadmin-course-short-name"
          name="shortName"
          type="text"
          value={shortName()}
          maxlength={200}
          autocomplete="off"
          required
          disabled={props.disabled}
          onInput={(event) => setShortName(event.currentTarget.value)}
        />
        <small>For compact navigation; about 16 characters when practical.</small>
      </label>
      <label for="sysadmin-course-long-name">
        Course long name
        <input
          id="sysadmin-course-long-name"
          name="longName"
          type="text"
          value={longName()}
          maxlength={200}
          autocomplete="off"
          required
          disabled={props.disabled}
          onInput={(event) => setLongName(event.currentTarget.value)}
        />
      </label>
      <CourseClassificationFields
        value={classification()}
        disabled={props.disabled}
        onChange={setClassification}
      />
      <label for="sysadmin-course-start-date">
        Course Term start date
        <input
          id="sysadmin-course-start-date"
          name="startDate"
          type="date"
          value={startDate()}
          required
          disabled={props.disabled}
          onInput={(event) => setStartDate(event.currentTarget.value)}
        />
      </label>
      <label for="sysadmin-course-end-date">
        Course Term end date
        <input
          id="sysadmin-course-end-date"
          name="endDate"
          type="date"
          value={endDate()}
          required
          disabled={props.disabled}
          onInput={(event) => setEndDate(event.currentTarget.value)}
        />
      </label>
      <button class="primary-action" type="submit" disabled={props.disabled || noInstructors()}>
        {props.disabled ? "Creating Course..." : "Create Course Instance"}
      </button>
      <Show when={props.error}>
        {(message) => (
          <p class="inline-error" role="alert">
            {message()}
          </p>
        )}
      </Show>
      <Show when={props.status}>{(message) => <p role="status">{message()}</p>}</Show>
    </form>
  );
}
