// Local Blueprint Course working state before one live create request.

import { useNavigate } from "@solidjs/router";
import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { CreateBlueprintCourseInput } from "../../../generated/api/CreateBlueprintCourseInput";
import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import {
  ASSESSMENT_TYPE_OPTIONS,
  assessmentTypePresentation,
  isAssessmentType,
} from "../../assessment_type_presentation";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { UnsavedChangesGuard } from "../../components/unsaved_changes_guard";
import {
  CourseClassificationFields,
  emptyCourseClassification,
  type CourseClassificationDraft,
} from "../../components/course_classification_fields";
import { decodeCourseClassification } from "../../api/decoders/course_classification";
import {
  QuestionPicker,
  type QuestionPickerSelection,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../question_picker";
import { createBlueprintCourseWhenReady } from "./blueprint_course_creation";
import { appendPickedFixedEntries, emptyReusableContent } from "./blueprint_course_model";

export interface BlueprintCourseCreateDialogProps {
  readonly client: BlueprintCourseClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onClose: () => void;
  readonly onFailure: (text: string) => void;
}

function detailPath(reference: string): string {
  return `/blueprint-courses/${encodeURIComponent(reference)}`;
}

/** Keeps incomplete Blueprint Course working state in the browser until it has reusable content. */
export function BlueprintCourseCreateDialog(props: BlueprintCourseCreateDialogProps): JSX.Element {
  const navigate = useNavigate();
  const [shortName, setShortName] = createSignal("Untitled Blueprint");
  const [longName, setLongName] = createSignal("Untitled Blueprint Course");
  const [classification, setClassification] = createSignal<CourseClassificationDraft>(
    emptyCourseClassification(),
  );
  const [moduleLabel, setModuleLabel] = createSignal("Module 1");
  const [assessmentTitle, setAssessmentTitle] = createSignal("Module 1 assessment");
  const [content, setContent] = createSignal<BlueprintAssessmentContentInput>();
  const [showPicker, setShowPicker] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
  const [closeRequested, setCloseRequested] = createSignal(false);
  const [message, setMessage] = createSignal(
    "Name the Blueprint Course, choose an Assessment Type, and choose its first published Question.",
  );
  let dialog!: HTMLDialogElement;
  let shortNameInput!: HTMLInputElement;
  let pickerTrigger: HTMLButtonElement | undefined;

  function closeCreation(): void {
    if (dialog.open) dialog.close();
    props.onClose();
  }

  function requestCloseCreation(): void {
    if (dirty()) setCloseRequested(true);
    else closeCreation();
  }

  function creationInput(): CreateBlueprintCourseInput | undefined {
    const assessment = content();
    if (assessment === undefined) return undefined;
    return {
      classification: decodeCourseClassification(classification()),
      short_name: shortName(),
      long_name: longName(),
      modules: [
        {
          label: moduleLabel(),
          assessments: [
            { ...assessment, title: assessmentTitle().trim() || "Module 1 assessment" },
          ],
        },
      ],
    };
  }

  function chooseQuestions(selection: QuestionPickerSelection): void {
    const current = content();
    if (current === undefined) {
      setMessage("Choose an Assessment Type before selecting Questions.");
      return;
    }
    setContent(appendPickedFixedEntries(current, selection));
    setDirty(true);
    setShowPicker(false);
    setMessage(
      `Added ${selection.questionIds.length} selected Question${selection.questionIds.length === 1 ? "" : "s"}. Review the local working state, then create the Blueprint Course.`,
    );
  }

  async function save(): Promise<void> {
    try {
      decodeCourseClassification(classification());
    } catch {
      setMessage(
        "Choose a Discipline and check that Tags are unique, trimmed labels of 1 through 120 characters.",
      );
      return;
    }
    const input = creationInput();
    if (input === undefined) {
      setMessage("Choose an Assessment Type before creating the Blueprint Course.");
      return;
    }
    setBusy(true);
    try {
      const result = await createBlueprintCourseWhenReady(props.client, input, crypto.randomUUID());
      if (result.kind === "invalid") {
        setMessage(result.message);
        return;
      }
      // The local working state is now persisted as Revision 1, so its guard
      // must not intercept this component's own successful redirect.
      setDirty(false);
      navigate(detailPath(result.value.blueprintCourse.reference));
    } catch (error: unknown) {
      const text =
        error instanceof Error
          ? error.message
          : "The Blueprint Course could not be created. This local working state remains ready to retry.";
      setMessage(text);
      props.onFailure(text);
    } finally {
      setBusy(false);
    }
  }

  onMount(() => {
    queueMicrotask(() => {
      dialog.showModal();
      shortNameInput.focus();
    });
  });

  onCleanup(() => {
    if (dialog.open) dialog.close();
  });

  return (
    <>
      <dialog
        class="blueprint-course-create-dialog"
        aria-labelledby="blueprint-course-create-heading"
        ref={(element) => {
          dialog = element;
        }}
        onCancel={(event) => {
          event.preventDefault();
          requestCloseCreation();
        }}
      >
        <div class="blueprint-course-section-heading">
          <div>
            <h2 id="blueprint-course-create-heading">Create a Blueprint Course</h2>
            <p>A Blueprint Course is reusable structure with no Students or delivery dates.</p>
          </div>
          <button
            type="button"
            class="quiet-action"
            disabled={busy()}
            onClick={requestCloseCreation}
          >
            Close creation
          </button>
        </div>
        <p class="blueprint-course-notice" role="status">
          {message()}
        </p>
        <label>
          Blueprint Course short name
          <input
            ref={(element) => {
              shortNameInput = element;
            }}
            value={shortName()}
            maxlength="200"
            onInput={(event) => {
              setShortName(event.currentTarget.value);
              setDirty(true);
            }}
          />
        </label>
        <label>
          Blueprint Course long name
          <input
            value={longName()}
            maxlength="200"
            onInput={(event) => {
              setLongName(event.currentTarget.value);
              setDirty(true);
            }}
          />
        </label>
        <CourseClassificationFields
          value={classification()}
          disabled={busy()}
          onChange={(value) => {
            setClassification(value);
            setDirty(true);
          }}
        />
        <label>
          First module label
          <input
            value={moduleLabel()}
            maxlength="200"
            onInput={(event) => {
              setModuleLabel(event.currentTarget.value);
              setDirty(true);
            }}
          />
        </label>
        <label>
          First Assessment Type
          <select
            required
            value={content()?.assessment_type ?? ""}
            onChange={(event) => {
              const assessmentType = event.currentTarget.value;
              if (!isAssessmentType(assessmentType)) return;
              setContent((current) =>
                current === undefined
                  ? emptyReusableContent(assessmentType, assessmentTitle())
                  : {
                      ...emptyReusableContent(assessmentType, current.title),
                      entries: current.entries,
                    },
              );
              setDirty(true);
            }}
          >
            <option value="" disabled>
              Choose an Assessment Type
            </option>
            <For each={ASSESSMENT_TYPE_OPTIONS}>
              {(option) => <option value={option.value}>{option.label}</option>}
            </For>
          </select>
        </label>
        <Show when={content()?.assessment_type}>
          {(selectedType) => (
            <p class="blueprint-course-field-help">
              {assessmentTypePresentation(selectedType()).description}
            </p>
          )}
        </Show>
        <p class="blueprint-course-field-help">
          Type describes the Assessment's teaching purpose. You can edit its reusable settings
          independently.
        </p>
        <label>
          First assessment title
          <input
            value={assessmentTitle()}
            maxlength="200"
            onInput={(event) => {
              const title = event.currentTarget.value;
              setAssessmentTitle(title);
              setContent((current) => (current === undefined ? undefined : { ...current, title }));
              setDirty(true);
            }}
          />
        </label>
        <p>
          {(content()?.entries.length ?? 0) === 0
            ? "No Questions selected yet."
            : `${content()?.entries.length ?? 0} fixed Question${content()?.entries.length === 1 ? "" : "s"} selected in order.`}
        </p>
        <button
          type="button"
          disabled={busy() || content() === undefined}
          onClick={(event) => {
            pickerTrigger = event.currentTarget;
            setShowPicker(true);
            setMessage(
              "Choose published Questions, confirm their order, then return to this local working state.",
            );
          }}
        >
          Choose published Questions
        </button>
        <div class="blueprint-course-save-actions">
          <button type="button" disabled={busy()} onClick={() => void save()}>
            {busy() ? "Creating..." : "Create Blueprint Course"}
          </button>
        </div>
        <Show when={showPicker()}>
          <QuestionPicker
            repository={props.pickerRepository}
            sources={props.pickerSources}
            mode="many"
            maximumSelection={1024}
            trigger={pickerTrigger}
            title="Choose the first reusable Questions"
            confirmLabel="Use selected Questions"
            onConfirm={chooseQuestions}
            onCancel={() => setShowPicker(false)}
          />
        </Show>
      </dialog>
      <UnsavedChangesGuard
        dirty={dirty}
        manualLeave={{
          requested: closeRequested,
          clearRequest: () => setCloseRequested(false),
          continue: closeCreation,
        }}
        save={() => {
          setMessage(
            "Finish this local Blueprint Course with a published Question, then create it.",
          );
          return Promise.resolve(false);
        }}
        copy={{
          heading: "Keep creating this Blueprint Course?",
          description:
            "Your local Blueprint Course has not been created as Revision 1 and will be discarded if you continue.",
          saveActionLabel: "Keep editing",
          savingActionLabel: "Checking local Blueprint Course...",
          saveFailureMessage:
            "This local Blueprint Course remains available while you keep editing.",
        }}
      />
    </>
  );
}
