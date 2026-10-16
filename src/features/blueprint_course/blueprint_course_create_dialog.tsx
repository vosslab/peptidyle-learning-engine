// Local Blueprint Course working state before one live create request.

import { useBeforeLeave, useNavigate } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { CreateBlueprintCourseInput } from "../../../generated/api/CreateBlueprintCourseInput";
import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { Theme } from "../../../generated/api/Theme";
import {
  ASSESSMENT_TYPE_OPTIONS,
  assessmentTypePresentation,
  isAssessmentType,
} from "../../assessment_type_presentation";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { UnsavedChangesGuard, trackEditorSave } from "../../components/unsaved_changes_guard";
import {
  CourseClassificationFields,
  emptyCourseClassification,
  type CourseClassificationDraft,
} from "../../components/course_classification_fields";
import { decodeCourseClassification } from "../../api/decoders/course_classification";
import { useAppearance } from "../../appearance/appearance_context";
import { ThemeChooser } from "../../appearance/theme_chooser";
import {
  QuestionPicker,
  type QuestionPickerProps,
  type QuestionPickerSelection,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../question_picker";
import { createBlueprintCourseWhenReady } from "./blueprint_course_creation";
import {
  appendPickedFixedEntries,
  emptyReusableContent,
  hasBlueprintCourseCreationChanges,
  type BlueprintCourseCreationDraft,
} from "./blueprint_course_model";

export interface BlueprintCourseCreateDialogProps {
  readonly client: BlueprintCourseClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly loadQuestionInspection: QuestionPickerProps["loadQuestionInspection"];
  readonly questionRevisionPreviewDocumentUrl: QuestionPickerProps["questionRevisionPreviewDocumentUrl"];
  readonly questionImageUrl: QuestionPickerProps["questionImageUrl"];
  readonly onClose: () => void;
  readonly onFailure: (text: string) => void;
}

function detailPath(blueprintCourseId: string): string {
  return `/blueprint-courses/${encodeURIComponent(blueprintCourseId)}`;
}

/** Keeps incomplete Blueprint Course working state in the browser until it has reusable content. */
export function BlueprintCourseCreateDialog(props: BlueprintCourseCreateDialogProps): JSX.Element {
  const navigate = useNavigate();
  const appearance = useAppearance();
  const initialDraft: BlueprintCourseCreationDraft = {
    shortName: "Untitled Blueprint",
    longName: "Untitled Blueprint Course",
    theme: "grass",
    classification: emptyCourseClassification(),
    moduleLabel: "Module 1",
    assessmentTitle: "Module 1 assessment",
    content: undefined,
  };
  const [shortName, setShortName] = createSignal(initialDraft.shortName);
  const [longName, setLongName] = createSignal(initialDraft.longName);
  const [theme, setTheme] = createSignal<Theme>(initialDraft.theme);
  const [classification, setClassification] = createSignal<CourseClassificationDraft>(
    initialDraft.classification,
  );
  const [moduleLabel, setModuleLabel] = createSignal(initialDraft.moduleLabel);
  const [assessmentTitle, setAssessmentTitle] = createSignal(initialDraft.assessmentTitle);
  const [content, setContent] = createSignal<BlueprintAssessmentContentInput>();
  const [showPicker, setShowPicker] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [created, setCreated] = createSignal(false);
  const dirty = createMemo(
    () =>
      !created() &&
      hasBlueprintCourseCreationChanges(
        {
          shortName: shortName(),
          longName: longName(),
          theme: theme(),
          classification: classification(),
          moduleLabel: moduleLabel(),
          assessmentTitle: assessmentTitle(),
          content: content(),
        },
        initialDraft,
      ),
  );
  const [closeRequested, setCloseRequested] = createSignal(false);
  const [message, setMessage] = createSignal(
    "Name the Blueprint Course, choose an Assessment Type, and choose its first published Question.",
  );
  let dialog!: HTMLDialogElement;
  let shortNameInput!: HTMLInputElement;
  let pickerTrigger: HTMLButtonElement | undefined;
  let createdPath: string | undefined;
  let leaveRequested = false;
  let disposed = false;

  useBeforeLeave(() => {
    if (dirty() || trackedCreate.inFlightSave() !== undefined) leaveRequested = true;
  });

  function closeCreation(): void {
    if (dialog.open) dialog.close();
    props.onClose();
  }

  function requestCloseCreation(): void {
    if (dirty() || trackedCreate.inFlightSave() !== undefined) {
      leaveRequested = true;
      setCloseRequested(true);
    } else closeCreation();
  }

  function creationInput(): CreateBlueprintCourseInput | undefined {
    const assessment = content();
    if (assessment === undefined) return undefined;
    return {
      classification: decodeCourseClassification(classification()),
      short_name: shortName(),
      long_name: longName(),
      theme: theme(),
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

  function chooseQuestions(selection: QuestionPickerSelection): boolean {
    const current = content();
    if (current === undefined) {
      setMessage("Choose an Assessment Type before selecting Questions.");
      return false;
    }
    setContent(appendPickedFixedEntries(current, selection));
    setShowPicker(false);
    setMessage(
      `Added ${selection.questionIds.length} selected Question${selection.questionIds.length === 1 ? "" : "s"}. Review the local working state, then create the Blueprint Course.`,
    );
    return true;
  }

  async function create(): Promise<boolean> {
    if (created()) return false;
    try {
      decodeCourseClassification(classification());
    } catch {
      setMessage(
        "Choose a Discipline and check that Tags are unique, trimmed labels of 1 through 120 characters.",
      );
      return false;
    }
    const input = creationInput();
    if (input === undefined) {
      setMessage("Choose an Assessment Type before creating the Blueprint Course.");
      return false;
    }
    setBusy(true);
    try {
      const result = await createBlueprintCourseWhenReady(props.client, input, crypto.randomUUID());
      if (result.kind === "invalid") {
        setMessage(result.message);
        return false;
      }
      // The local working state is now persisted as Revision 1, so its guard
      // must not intercept this component's own successful redirect.
      setCreated(true);
      createdPath = detailPath(result.value.blueprintCourse.id);
      setMessage("Blueprint Course created as Revision 1. Close creation to continue.");
      return true;
    } catch (error: unknown) {
      const text =
        error instanceof Error
          ? error.message
          : "The Blueprint Course could not be created. This local working state remains ready to retry.";
      if (!disposed) {
        setMessage(text);
        props.onFailure(text);
      }
      return false;
    } finally {
      setBusy(false);
    }
  }

  const trackedCreate = trackEditorSave(create);

  function save(): void {
    if (created() || trackedCreate.inFlightSave() !== undefined) return;
    leaveRequested = false;
    void trackedCreate.save().then((saved) => {
      if (saved && !disposed && !leaveRequested && createdPath !== undefined) navigate(createdPath);
    });
  }

  onMount(() => {
    queueMicrotask(() => {
      dialog.showModal();
      shortNameInput.focus();
    });
  });

  onCleanup(() => {
    disposed = true;
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
        <label for="blueprint-course-create-short-name">
          Blueprint Course short name
          <input
            id="blueprint-course-create-short-name"
            ref={(element) => {
              shortNameInput = element;
            }}
            value={shortName()}
            disabled={created()}
            maxlength="200"
            aria-describedby="blueprint-course-create-short-name-help"
            onInput={(event) => {
              setShortName(event.currentTarget.value);
            }}
          />
        </label>
        <small id="blueprint-course-create-short-name-help">
          For compact navigation; about 16 characters when practical.
        </small>
        <label>
          Blueprint Course long name
          <input
            value={longName()}
            disabled={created()}
            maxlength="200"
            onInput={(event) => {
              setLongName(event.currentTarget.value);
            }}
          />
        </label>
        <CourseClassificationFields
          value={classification()}
          disabled={busy() || created()}
          onChange={(value) => {
            setClassification(value);
          }}
        />
        <fieldset disabled={busy() || created()}>
          <legend>Starting Blueprint Theme</legend>
          <ThemeChooser
            name="blueprint-course-theme"
            selectedTheme={theme}
            mode={() => appearance.appearance().mode}
            disabled={() => busy() || created()}
            onSelect={(value) => {
              setTheme(value);
            }}
          />
        </fieldset>
        <label>
          First module label
          <input
            value={moduleLabel()}
            disabled={created()}
            maxlength="200"
            onInput={(event) => {
              setModuleLabel(event.currentTarget.value);
            }}
          />
        </label>
        <label>
          First Assessment Type
          <select
            required
            value={content()?.assessment_type ?? ""}
            disabled={created()}
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
            disabled={created()}
            maxlength="200"
            onInput={(event) => {
              const title = event.currentTarget.value;
              setAssessmentTitle(title);
              setContent((current) => (current === undefined ? undefined : { ...current, title }));
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
          disabled={busy() || created() || content() === undefined}
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
          <button type="button" disabled={busy() || created()} onClick={save}>
            {busy() ? "Creating..." : "Create Blueprint Course"}
          </button>
        </div>
        <Show when={showPicker()}>
          <QuestionPicker
            repository={props.pickerRepository}
            sources={props.pickerSources}
            loadQuestionInspection={props.loadQuestionInspection}
            questionRevisionPreviewDocumentUrl={props.questionRevisionPreviewDocumentUrl}
            questionImageUrl={props.questionImageUrl}
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
        inFlightSave={trackedCreate.inFlightSave}
        manualLeave={{
          requested: closeRequested,
          clearRequest: () => setCloseRequested(false),
          continue: closeCreation,
        }}
        copy={{
          heading: "Keep creating this Blueprint Course?",
          description:
            "Your local Blueprint Course has not been created as Revision 1 and will be discarded if you continue.",
        }}
      />
    </>
  );
}
