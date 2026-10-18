// course_instance_page.tsx - Instructor Course Instance teaching workspace.

import { A, useNavigate, useParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, Show, untrack, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { CourseInstanceSummary } from "../api/course_instance";
import { decodeCreateBlueprintFromCourseInstanceInput } from "../api/decoders/course_instance";
import { ApiRequestError } from "../api/http_client";
import { CourseClassificationEditor } from "../components/course_classification_editor";
import {
  CourseClassificationFields,
  type CourseClassificationDraft,
} from "../components/course_classification_fields";
import { CourseStudentWorkRecovery } from "../components/course_student_work_recovery";
import { PageFrame } from "../components/page_frame/page_frame";
import { UnsavedChangesGuard } from "../components/unsaved_changes_guard";
import type { RecordContent } from "../components/data_sheet/record_content";
import { formatLocalWallClockDateTime } from "../format_datetime";
import type { CourseAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import { LiveAssessmentWorkspaceConflictError } from "../api/http_client/assessment_release";
import { parseCourseInstanceId } from "../navigation/public_route";
import { CompactShortNameField } from "./compact_short_name_field";
import {
  assessmentCollectionFields,
  type AssessmentCollectionRow,
} from "./course_collection_fields";
import { CourseBlueprintUpdateReviewList } from "./course_blueprint_update_review";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import {
  canonicalLocalDateAndTime,
  dueDateDraft,
  dueTimeDraft,
  localDueDateAndTime,
} from "./assessment_workspace/assessment_workspace_policy_model";

import "./course_instance_page.css";

function assessmentStatusLabel(status: LiveAssessmentStatus): string {
  switch (status) {
    case "unreleased":
      return "Unreleased";
    case "released":
      return "Released";
    case "closed":
      return "Closed";
    case "archived":
      return "Archived";
  }
}

function assessmentQuestionsPath(courseInstanceId: string, assessmentId: string): string {
  // ASVS 1.2.2: encode each API-supplied identity as one URL path segment.
  return `/instructor/courses/${encodeURIComponent(courseInstanceId)}/assessments/${encodeURIComponent(assessmentId)}/questions`;
}

function formatAssessmentDueDate(value: CourseAssessmentSummary["dueAt"]): string {
  if (value === null) return "No due date";
  return formatLocalWallClockDateTime(value);
}

/** Creates a distinct Active Blueprint while retaining the source Course Instance unchanged. */
function CreateBlueprintFromCourseInstance(props: {
  readonly course: CourseInstanceSummary;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const [open, setOpen] = createSignal(false);
  const [shortName, setShortName] = createSignal(props.course.shortName);
  const [longName, setLongName] = createSignal(props.course.longName);
  const [classification, setClassification] = createSignal<CourseClassificationDraft>(
    props.course.classification,
  );
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  let dialog: HTMLDialogElement | undefined;
  let opener: HTMLButtonElement | undefined;
  let shortNameInput: HTMLInputElement | undefined;
  let creationAction: { readonly payload: string; readonly key: string } | undefined;

  function close(): void {
    if (busy()) return;
    if (dialog?.open) dialog.close();
    setOpen(false);
    queueMicrotask(() => opener?.focus());
  }

  function openDialog(): void {
    setShortName(props.course.shortName);
    setLongName(props.course.longName);
    setClassification(props.course.classification);
    setMessage("");
    setOpen(true);
    queueMicrotask(() => {
      dialog?.showModal();
      shortNameInput?.focus();
    });
  }

  async function create(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy()) return;
    let input;
    try {
      input = decodeCreateBlueprintFromCourseInstanceInput({
        classification: classification(),
        shortName: shortName(),
        longName: longName(),
      });
    } catch {
      setMessage(
        "Enter trimmed Blueprint short and long names, choose a Discipline, and check the classification.",
      );
      shortNameInput?.focus();
      return;
    }
    const payload = JSON.stringify(input);
    if (creationAction?.payload !== payload) {
      creationAction = { payload, key: crypto.randomUUID() };
    }
    setBusy(true);
    setMessage("");
    try {
      const result = await applicationApi.client.createBlueprintFromCourseInstance(
        props.course.id,
        input,
        creationAction.key,
      );
      creationAction = undefined;
      if (dialog?.open) dialog.close();
      navigate(`/blueprint-courses/${encodeURIComponent(result.blueprintCourse.id)}`);
    } catch (error: unknown) {
      setMessage(
        error instanceof ApiRequestError && error.status === 401
          ? "Your session ended. Sign in again, then return to this Course Instance."
          : "The Blueprint could not be confirmed. Retry to confirm the same creation request.",
      );
      queueMicrotask(() => shortNameInput?.focus());
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <button
        ref={(element) => {
          opener = element;
        }}
        class="quiet-action"
        type="button"
        onClick={openDialog}
      >
        Create Blueprint from Course Instance
      </button>
      <Show when={open()}>
        <dialog
          class="course-instance-blueprint-dialog"
          aria-labelledby="course-instance-blueprint-heading"
          aria-describedby="course-instance-blueprint-help"
          ref={(element) => {
            dialog = element;
          }}
          onCancel={(event) => {
            event.preventDefault();
            close();
          }}
        >
          <header class="course-instance-blueprint-dialog__heading">
            <div>
              <h2 id="course-instance-blueprint-heading">Create Blueprint from Course Instance</h2>
              <p id="course-instance-blueprint-help">
                Creates a new Blueprint by copying reusable structure. This Course Instance remains
                unchanged and becomes the new Blueprint Course's first Adoption.
              </p>
            </div>
          </header>
          <form aria-busy={busy()} onSubmit={(event) => void create(event)}>
            <fieldset disabled={busy()}>
              <div class="course-instance-blueprint-dialog__fields">
                <CompactShortNameField
                  label="Blueprint short name"
                  name="shortName"
                  value={shortName()}
                  maxLength={200}
                  helpId="course-instance-blueprint-short-name-help"
                  inputRef={(element) => {
                    shortNameInput = element;
                  }}
                  onInput={setShortName}
                />
                <label>
                  Blueprint long name
                  <input
                    name="longName"
                    value={longName()}
                    maxlength="200"
                    autocomplete="off"
                    required
                    onInput={(event) => setLongName(event.currentTarget.value)}
                  />
                </label>
              </div>
              <CourseClassificationFields
                value={classification()}
                disabled={busy()}
                onChange={setClassification}
              />
              <div class="course-instance-blueprint-dialog__actions">
                <button class="primary-action" type="submit">
                  {busy() ? "Creating Blueprint..." : "Create Blueprint"}
                </button>
                <button class="quiet-action" type="button" onClick={close}>
                  Cancel
                </button>
              </div>
            </fieldset>
          </form>
          <Show when={busy()}>
            <p role="status" aria-live="polite" aria-atomic="true">
              Creating Blueprint from Course Instance...
            </p>
          </Show>
          <Show when={message()}>{(value) => <p role="alert">{value()}</p>}</Show>
        </dialog>
      </Show>
    </>
  );
}

type AssessmentEditorState = "idle" | "editing" | "saving" | "failed" | "read-only";

export type AssessmentRowModel = {
  readonly content: (position: number) => RecordContent;
  readonly details: () => JSX.Element;
  readonly detailsVisible: () => boolean;
  readonly refresh: (assessment: CourseAssessmentSummary) => void;
  readonly dirty: () => boolean;
  readonly inFlightSave: () => Promise<boolean> | undefined;
};

type AssessmentRowClient = Pick<
  ReturnType<typeof useApplicationApi>["client"],
  "listCourseAssessments" | "saveLiveAssessmentInline"
>;

export function createAssessmentRowModel(props: {
  readonly courseInstanceId: string;
  readonly assessment: CourseAssessmentSummary;
  readonly client: AssessmentRowClient;
  readonly onSaved?: (assessment: CourseAssessmentSummary) => void;
}): AssessmentRowModel {
  const [assessment, setAssessment] = createSignal(props.assessment);
  const [state, setState] = createSignal<AssessmentEditorState>("idle");
  const [title, setTitle] = createSignal(assessment().title);
  const [dueDate, setDueDate] = createSignal(dueDateDraft(assessment().dueAt));
  const [dueTime, setDueTime] = createSignal(dueTimeDraft(assessment().dueAt));
  const [message, setMessage] = createSignal("");
  const [needsRefresh, setNeedsRefresh] = createSignal(false);
  const [inFlightSave, setInFlightSave] = createSignal<Promise<boolean> | undefined>();
  let expectedAssessmentEditNumber = props.assessment.assessmentEditNumber;
  let titleInput: HTMLInputElement | undefined;
  let editButton: HTMLButtonElement | undefined;
  let assessmentQuestionsLink: HTMLAnchorElement | undefined;
  let readOnlyNotice: HTMLParagraphElement | undefined;

  function mayEdit(): boolean {
    return assessment().status === "unreleased" || assessment().status === "released";
  }

  function beginEditing(): void {
    const current = assessment();
    expectedAssessmentEditNumber = current.assessmentEditNumber;
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setNeedsRefresh(false);
    setState("editing");
    queueMicrotask(() => titleInput?.focus());
  }

  function cancelEditing(): void {
    const current = assessment();
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setNeedsRefresh(false);
    setState("idle");
    queueMicrotask(focusAssessmentAction);
  }

  function focusAssessmentAction(): void {
    const focusTarget = mayEdit() ? editButton : assessmentQuestionsLink;
    focusTarget?.focus();
  }

  function closeReadOnlyEditor(): void {
    const current = assessment();
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setState("idle");
    queueMicrotask(focusAssessmentAction);
  }

  function typedDueDateAndTime(): string {
    if (dueDate() === "") return "No due date";
    return `${dueDate()} ${dueTime()}`;
  }

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const dueAt = canonicalLocalDateAndTime(localDueDateAndTime(dueDate(), dueTime()));
    if (dueDate() !== "" && dueAt === null) {
      setState("failed");
      setMessage("Enter a complete local due date and time, or clear the due date.");
      return;
    }
    setState("saving");
    setMessage("");
    const operation = (async (): Promise<boolean> => {
      try {
        const saved = await props.client.saveLiveAssessmentInline(
          props.courseInstanceId,
          assessment().id,
          { title: title(), dueAt },
          expectedAssessmentEditNumber,
        );
        setAssessment(saved);
        setTitle(saved.title);
        setDueDate(dueDateDraft(saved.dueAt));
        setDueTime(dueTimeDraft(saved.dueAt));
        props.onSaved?.(saved);
        expectedAssessmentEditNumber = saved.assessmentEditNumber;
        setState("idle");
        setMessage("Assessment title and due date saved.");
        queueMicrotask(() => editButton?.focus());
        return true;
      } catch (error: unknown) {
        setState("failed");
        setNeedsRefresh(error instanceof LiveAssessmentWorkspaceConflictError);
        setMessage(
          error instanceof LiveAssessmentWorkspaceConflictError
            ? "This Assessment changed elsewhere. Load the current Assessment, then review it before retrying. Your typed values remain here."
            : "The Assessment was not saved. Check the title and local due date and time, then try again. Your typed values remain here.",
        );
        return false;
      }
    })();
    void setInFlightSave(operation);
    await operation;
    setInFlightSave(undefined);
  }

  async function refreshCurrentAssessment(): Promise<void> {
    setState("saving");
    setMessage("");
    try {
      const latest = (await props.client.listCourseAssessments(props.courseInstanceId)).find(
        (candidate) => candidate.id === assessment().id,
      );
      if (latest === undefined) throw new Error("Current Assessment was not returned");
      setAssessment(latest);
      expectedAssessmentEditNumber = latest.assessmentEditNumber;
      setNeedsRefresh(false);
      if (!mayEdit()) {
        setState("read-only");
        setMessage(
          `This Assessment is ${assessmentStatusLabel(latest.status)} and cannot save title and due date edits. Your typed values remain available until you close this editor.`,
        );
        queueMicrotask(() => readOnlyNotice?.focus());
        return;
      }
      setState("failed");
      setMessage("Latest Assessment loaded. Review it, then save your typed values again.");
      queueMicrotask(() => titleInput?.focus());
    } catch {
      setState("failed");
      setMessage(
        "The current Assessment could not load. Your typed values remain here. Try again.",
      );
    }
  }

  return {
    inFlightSave,
    dirty: () =>
      state() === "saving" ||
      title() !== assessment().title ||
      dueDate() !== dueDateDraft(assessment().dueAt) ||
      dueTime() !== dueTimeDraft(assessment().dueAt),
    refresh: (refreshedAssessment): void => {
      setAssessment(refreshedAssessment);
      if (state() === "idle") {
        setTitle(refreshedAssessment.title);
        setDueDate(dueDateDraft(refreshedAssessment.dueAt));
        setDueTime(dueTimeDraft(refreshedAssessment.dueAt));
        expectedAssessmentEditNumber = refreshedAssessment.assessmentEditNumber;
      }
    },
    content: (position): RecordContent => {
      const current = assessment();
      return {
        title: current.title,
        description: `Assessment ${position}`,
        details: [
          { kind: "assessmentType", value: current.assessmentType },
          { kind: "text", label: "Status:", value: assessmentStatusLabel(current.status) },
          current.dueAt === null
            ? { kind: "text", label: "Due:", value: "No due date" }
            : {
                kind: "time",
                label: "Due:",
                value: formatAssessmentDueDate(current.dueAt),
                dateTime: current.dueAt,
              },
        ],
        actions: [
          {
            id: "edit-assessment",
            kind: "link",
            label: "Edit Assessment",
            href: assessmentQuestionsPath(props.courseInstanceId, current.id),
            ref: (element) => (assessmentQuestionsLink = element),
          },
          ...(mayEdit() && state() === "idle"
            ? [
                {
                  id: "edit-title-and-due-date",
                  kind: "command" as const,
                  label: "Edit title and due date",
                  onClick: () => beginEditing(),
                  ref: (element: HTMLButtonElement) => (editButton = element),
                },
              ]
            : []),
        ],
      };
    },
    detailsVisible: () => state() !== "idle" || message() !== "",
    details: () => (
      <>
        <Show when={state() !== "idle" && state() !== "read-only"}>
          <form
            class="assessment-inline-editor"
            aria-label={`Edit ${assessment().title}`}
            aria-busy={state() === "saving"}
            onSubmit={(event) => void save(event)}
          >
            <fieldset disabled={state() === "saving"}>
              <div class="assessment-inline-editor__fields">
                <label class="assessment-inline-editor__field">
                  Title
                  <input
                    ref={(element) => (titleInput = element)}
                    value={title()}
                    onInput={(event) => setTitle(event.currentTarget.value)}
                    required
                  />
                </label>
                <div
                  class="assessment-inline-editor__schedule"
                  role="group"
                  aria-label="Due date and time"
                >
                  <label class="assessment-inline-editor__field">
                    Due date
                    <input
                      type="date"
                      value={dueDate()}
                      onInput={(event) => setDueDate(event.currentTarget.value)}
                    />
                  </label>
                  <label class="assessment-inline-editor__field">
                    Due time
                    <input
                      type="time"
                      step="0.001"
                      value={dueTime()}
                      onInput={(event) => setDueTime(event.currentTarget.value)}
                    />
                  </label>
                </div>
              </div>
              <div class="assessment-inline-editor__actions">
                <button class="primary-action" type="submit">
                  {state() === "saving"
                    ? "Saving title and due date..."
                    : "Save title and due date"}
                </button>
                <button class="quiet-action" type="button" onClick={cancelEditing}>
                  Cancel
                </button>
                <Show when={needsRefresh()}>
                  <button
                    class="quiet-action"
                    type="button"
                    onClick={() => void refreshCurrentAssessment()}
                  >
                    Load current Assessment
                  </button>
                </Show>
              </div>
            </fieldset>
            <Show when={message()}>
              {(value) => (
                <p
                  class="assessment-inline-editor__message"
                  role={state() === "failed" ? "alert" : "status"}
                >
                  {value()}
                </p>
              )}
            </Show>
          </form>
        </Show>
        <Show when={state() === "read-only"}>
          <section
            class="assessment-inline-editor"
            aria-label={`Editing unavailable for ${assessment().title}`}
          >
            <p
              ref={(element) => (readOnlyNotice = element)}
              class="assessment-inline-editor__message"
              role="alert"
              tabindex="-1"
            >
              {message()}
            </p>
            <p class="assessment-inline-editor__message">Unsaved title: {title()}</p>
            <p class="assessment-inline-editor__message">
              Unsaved due date and time: {typedDueDateAndTime()}
            </p>
            <button class="quiet-action" type="button" onClick={closeReadOnlyEditor}>
              Close editor
            </button>
          </section>
        </Show>
        <Show when={state() === "idle" && message()}>
          {(value) => (
            <p class="assessment-inline-editor__message" role="status">
              {value()}
            </p>
          )}
        </Show>
      </>
    ),
  };
}

/** Owns one Course Instance's resources, search session, and inline Assessment drafts. */
function CourseInstanceContent(props: { readonly courseInstanceId: string }): JSX.Element {
  const applicationApi = useApplicationApi();
  const assessmentRowModels = new Map<string, AssessmentRowModel>();
  function courseInstanceId(): string {
    return props.courseInstanceId;
  }
  const [course, { mutate: mutateCourse }] = createResource(courseInstanceId, async (id) =>
    applicationApi.client.getCourseInstance(id),
  );
  const [assessments, { refetch: refetchAssessments, mutate: mutateAssessments }] = createResource(
    courseInstanceId,
    async (id) =>
      (await applicationApi.client.listCourseAssessments(id)).map((assessment, index) => ({
        ...assessment,
        courseInstanceId: id,
        authoredPosition: index + 1,
      })),
  );
  const [profile, { refetch: refetchProfile }] = createResource(() =>
    applicationApi.client.getProfile(),
  );

  const assessmentRows = createMemo(() => {
    const id = courseInstanceId();
    const summaries = (assessments() ?? []).filter(
      (assessment) => assessment.courseInstanceId === id,
    );
    return untrack(() => {
      const presentIds = new Set(summaries.map((assessment) => assessment.id));
      for (const assessmentId of assessmentRowModels.keys()) {
        if (!presentIds.has(assessmentId)) assessmentRowModels.delete(assessmentId);
      }
      for (const assessment of summaries) {
        let model = assessmentRowModels.get(assessment.id);
        if (model === undefined) {
          model = createAssessmentRowModel({
            client: applicationApi.client,
            assessment,
            courseInstanceId: id,
            onSaved: (saved) =>
              mutateAssessments((current) =>
                current?.map((row) =>
                  row.id === saved.id
                    ? {
                        ...saved,
                        courseInstanceId: row.courseInstanceId,
                        authoredPosition: row.authoredPosition,
                      }
                    : row,
                ),
              ),
          });
          assessmentRowModels.set(assessment.id, model);
        } else {
          model.refresh(assessment);
        }
      }
      return summaries;
    });
  });
  const assessmentSearch = createLocalCompleteSearch(
    {
      rows: assessments,
      loading: () => assessments.loading,
      error: (): unknown => assessments.error,
      refetchSource: refetchAssessments,
    },
    {
      fields: assessmentCollectionFields,
      rowId: (assessment) => assessment.id,
      inScope: (assessment) => assessment.courseInstanceId === courseInstanceId(),
      keyword: (assessment) =>
        `${assessment.title} ${assessment.assessmentType} ${assessment.status} ${assessment.dueAt ?? ""}`,
      content: (assessment: AssessmentCollectionRow) => {
        assessmentRows();
        return assessmentRowModels.get(assessment.id)!.content(assessment.authoredPosition);
      },
    },
  );
  const inFlightSave = createMemo(() => {
    assessmentRows();
    const pending = [...assessmentRowModels.values()]
      .map((model) => model.inFlightSave())
      .filter((operation): operation is Promise<boolean> => operation !== undefined);
    return pending.length === 0
      ? undefined
      : Promise.all(pending).then((results) => results.every(Boolean));
  });

  return (
    <>
      <UnsavedChangesGuard
        dirty={() => {
          assessmentRows();
          return [...assessmentRowModels.values()].some((model) => model.dirty());
        }}
        inFlightSave={inFlightSave}
        copy={{
          heading: "Discard Assessment changes?",
          description: "Your Assessment title or due date changes have not been saved.",
        }}
      />
      <Show when={course.loading}>
        <PageFrame routeSurface="courseInstanceLoading">
          <p class="loading-state">Loading Course Instance...</p>
        </PageFrame>
      </Show>
      <Show
        when={course.error === undefined ? course() : undefined}
        fallback={
          <Show when={course.error !== undefined}>
            <PageFrame routeSurface="courseInstanceUnavailable">
              <section class="route-error" role="alert">
                <p>
                  This Course Instance is not available through your current Teaching Team access.
                </p>
                <A class="primary-link" href="/">
                  Return to Course Instances
                </A>
              </section>
            </PageFrame>
          </Show>
        }
      >
        {(view) => (
          <PageFrame
            routeSurface="courseInstance"
            actions={
              <A
                class="primary-link"
                href={`/instructor/courses/${view().courseInstance.id}/assessments/new`}
              >
                Create Assessment
              </A>
            }
          >
            <section aria-label="Assessments">
              <SearchBlock
                layout="above"
                controls={
                  <SearchControls state={assessmentSearch.state} textLabel="Search Assessments" />
                }
                state={assessmentSearch.state}
                collection={{
                  ariaLabel: "Assessments",
                  ...assessmentSearch.collection,
                  renderBody: (row) => {
                    assessmentRows();
                    const model = assessmentRowModels.get(row().id)!;
                    return <Show when={model.detailsVisible()}>{model.details()}</Show>;
                  },
                  get emptyState() {
                    return {
                      title:
                        assessmentSearch.state.query().text ||
                        assessmentSearch.state.query().filters.size > 0
                          ? "No Assessments match these filters"
                          : "No Assessments yet",
                      message:
                        "Assessments organize the ordered activities delivered to Students. Use Create Assessment to add the first activity for this Course Instance.",
                    };
                  },
                }}
              />
            </section>
            <section class="course-instance-page__details" aria-labelledby="course-details-heading">
              <h2 id="course-details-heading">Course details</h2>
              <div class="course-instance-page__detail-grid">
                <section
                  class="course-instance-page__detail-group"
                  aria-labelledby="course-term-heading"
                >
                  <h3 id="course-term-heading">Course Term</h3>
                  <p>
                    {view().courseInstance.term.startDate} through{" "}
                    {view().courseInstance.term.endDate}
                  </p>
                </section>
                <section
                  class="course-instance-page__detail-group"
                  aria-labelledby="teaching-team-heading"
                >
                  <h3 id="teaching-team-heading">Teaching Team</h3>
                  <p>You are an active co-Instructor for this Course Instance.</p>
                  <p>
                    {view().activeInstructorCount} active Instructor
                    {view().activeInstructorCount === 1 ? " is" : "s are"} currently recorded.
                  </p>
                </section>
                <section
                  class="course-instance-page__detail-group course-instance-page__detail-group--wide"
                  aria-labelledby="course-classification-heading"
                >
                  <h3 id="course-classification-heading">Classification</h3>
                  <CourseClassificationEditor
                    value={view().courseInstance.classification}
                    editNumber={view().courseInstance.courseEditNumber}
                    canEdit
                    save={async (classification, editNumber) => {
                      const saved = await applicationApi.client.updateCourseInstanceClassification(
                        view().courseInstance.id,
                        classification,
                        editNumber,
                      );
                      mutateCourse({
                        ...view(),
                        courseInstance: {
                          ...view().courseInstance,
                          classification: saved.classification,
                          courseEditNumber: saved.courseEditNumber,
                        },
                      });
                    }}
                    reload={async () => {
                      const current = await applicationApi.client.getCourseInstance(
                        view().courseInstance.id,
                      );
                      mutateCourse(current);
                      return {
                        classification: current.courseInstance.classification,
                        editNumber: current.courseInstance.courseEditNumber,
                      };
                    }}
                  />
                </section>
                <Show when={view().blueprintOrigin}>
                  {(origin) => (
                    <section
                      class="course-instance-page__detail-group course-instance-page__detail-group--wide"
                      aria-labelledby="blueprint-source-heading"
                    >
                      <h3 id="blueprint-source-heading">Blueprint source</h3>
                      <p data-blueprint-origin>
                        Adopted from Blueprint{" "}
                        <A
                          href={`/blueprint-courses/${origin().adoptedBlueprintCourseRevisionTuple.blueprintCourseId}`}
                        >
                          {origin().adoptedBlueprintCourseRevisionTuple.blueprintCourseId}
                        </A>
                        , Revision {origin().adoptedBlueprintCourseRevisionTuple.revisionNumber};
                        source now Revision{" "}
                        {origin().currentBlueprintCourseRevisionTuple.revisionNumber}.
                      </p>
                      <Show
                        when={
                          BigInt(origin().currentBlueprintCourseRevisionTuple.revisionNumber) >
                          BigInt(origin().adoptedBlueprintCourseRevisionTuple.revisionNumber)
                        }
                      >
                        <p data-blueprint-revision-notice>Newer Blueprint Revision available</p>
                        <CourseBlueprintUpdateReviewList
                          courseInstanceId={view().courseInstance.id}
                        />
                      </Show>
                    </section>
                  )}
                </Show>
              </div>
              <section
                class="course-instance-page__administration"
                aria-labelledby="course-administration-heading"
              >
                <h3 id="course-administration-heading">Course tools</h3>
                <nav class="course-instance-page__actions" aria-label="Course actions">
                  <A
                    class="quiet-link"
                    href={`/instructor/courses/${view().courseInstance.id}/students`}
                  >
                    Open Students
                  </A>
                  <A
                    class="quiet-link"
                    href={`/instructor/courses/${view().courseInstance.id}/gradebook`}
                  >
                    Gradebook
                  </A>
                  <A
                    class="quiet-link"
                    href={`/instructor/courses/${view().courseInstance.id}/appearance`}
                  >
                    Appearance
                  </A>
                </nav>
                <CreateBlueprintFromCourseInstance course={view().courseInstance} />
                <Show
                  when={profile.error === undefined}
                  fallback={
                    <section aria-label="Course date display unavailable">
                      <p role="alert">Course dates are unavailable. Refresh to try again.</p>
                      <button
                        type="button"
                        class="quiet-button"
                        onClick={() => void refetchProfile()}
                      >
                        Retry Course dates
                      </button>
                    </section>
                  }
                >
                  <Show when={profile()} fallback={<p role="status">Loading Course dates...</p>}>
                    {(settings) => (
                      <CourseStudentWorkRecovery
                        courseInstanceId={view().courseInstance.id}
                        client={applicationApi.client}
                        displayTimeZone={settings().timeZone}
                      />
                    )}
                  </Show>
                </Show>
              </section>
            </section>
          </PageFrame>
        )}
      </Show>
    </>
  );
}

/** Instructor Course Instance workspace for Teaching Team, roster, and Assessment delivery. */
export function CourseInstancePage(): JSX.Element {
  const params = useParams();
  return (
    <Show when={parseCourseInstanceId(params["courseInstanceId"] ?? "")} keyed>
      {(courseInstanceId) => <CourseInstanceContent courseInstanceId={courseInstanceId} />}
    </Show>
  );
}
