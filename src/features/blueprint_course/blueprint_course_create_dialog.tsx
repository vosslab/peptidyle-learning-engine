// Local Blueprint Course draft gate before one live create request.

import { useNavigate } from "@solidjs/router";
import { Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { CreateBlueprintCourseDefinitionInput } from "../../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { BlueprintAssignmentDefinitionInput } from "../../../generated/api/BlueprintAssignmentDefinitionInput";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import {
  QuestionPicker,
  type QuestionPickerSelection,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../question_picker";
import { createBlueprintCourseWhenReady } from "./blueprint_course_creation";
import { appendPickedFixedEntries, emptyReusableDefinition } from "./blueprint_course_model";

export interface CurriculumCreateDialogProps {
  readonly client: BlueprintCourseClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onClose: () => void;
  readonly onFailure: (text: string) => void;
}

function detailPath(reference: string): string {
  return `/curriculum/${encodeURIComponent(reference)}`;
}

/** Keeps an incomplete Blueprint Course draft in the browser until it has reusable content. */
export function CurriculumCreateDialog(props: CurriculumCreateDialogProps): JSX.Element {
  const navigate = useNavigate();
  const [title, setTitle] = createSignal("Untitled Blueprint Course");
  const [moduleLabel, setModuleLabel] = createSignal("Module 1");
  const [definition, setDefinition] = createSignal<BlueprintAssignmentDefinitionInput>(
    emptyReusableDefinition("Module 1 assignment"),
  );
  const [showPicker, setShowPicker] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal(
    "Name the Blueprint Course and choose its first published Question.",
  );
  let dialog!: HTMLDialogElement;
  let titleInput!: HTMLInputElement;
  let pickerTrigger: HTMLButtonElement | undefined;

  function closeDraft(): void {
    if (dialog.open) dialog.close();
    props.onClose();
  }

  function draft(): CreateBlueprintCourseDefinitionInput {
    return {
      title: title(),
      modules: [
        {
          label: moduleLabel(),
          definitions: [
            { ...definition(), title: definition().title.trim() || "Module 1 assignment" },
          ],
        },
      ],
    };
  }

  function chooseQuestions(selection: QuestionPickerSelection): void {
    setDefinition((current) => appendPickedFixedEntries(current, selection));
    setShowPicker(false);
    setMessage(
      `Added ${selection.questionIds.length} selected Question${selection.questionIds.length === 1 ? "" : "s"}. Review the draft, then create the Blueprint Course.`,
    );
  }

  async function save(): Promise<void> {
    setBusy(true);
    try {
      const result = await createBlueprintCourseWhenReady(props.client, draft());
      if (result.kind === "invalid") {
        setMessage(result.message);
        return;
      }
      navigate(detailPath(result.value.blueprintCourse.reference));
    } catch (error: unknown) {
      const text =
        error instanceof Error
          ? error.message
          : "The Blueprint Course could not be created. This local draft remains ready to retry.";
      setMessage(text);
      props.onFailure(text);
    } finally {
      setBusy(false);
    }
  }

  onMount(() => {
    queueMicrotask(() => {
      dialog.showModal();
      titleInput.focus();
    });
  });

  onCleanup(() => {
    if (dialog.open) dialog.close();
  });

  return (
    <dialog
      class="curriculum-create-dialog"
      aria-labelledby="curriculum-create-heading"
      ref={(element) => {
        dialog = element;
      }}
      onCancel={(event) => {
        event.preventDefault();
        closeDraft();
      }}
    >
      <div class="curriculum-section-heading">
        <div>
          <h2 id="curriculum-create-heading">Create a Blueprint Course</h2>
          <p>A Blueprint Course is reusable structure with no Students or delivery dates.</p>
        </div>
        <button type="button" class="quiet-action" disabled={busy()} onClick={closeDraft}>
          Close draft
        </button>
      </div>
      <p class="curriculum-notice" role="status">
        {message()}
      </p>
      <label>
        Blueprint Course title
        <input
          ref={(element) => {
            titleInput = element;
          }}
          value={title()}
          maxlength="200"
          onInput={(event) => setTitle(event.currentTarget.value)}
        />
      </label>
      <label>
        First module label
        <input
          value={moduleLabel()}
          maxlength="200"
          onInput={(event) => setModuleLabel(event.currentTarget.value)}
        />
      </label>
      <label>
        First assignment title
        <input
          value={definition().title}
          maxlength="200"
          onInput={(event) =>
            setDefinition((current) => ({ ...current, title: event.currentTarget.value }))
          }
        />
      </label>
      <p>
        {definition().entries.length === 0
          ? "No Questions selected yet."
          : `${definition().entries.length} fixed Question${definition().entries.length === 1 ? "" : "s"} selected in order.`}
      </p>
      <button
        type="button"
        disabled={busy()}
        onClick={(event) => {
          pickerTrigger = event.currentTarget;
          setShowPicker(true);
          setMessage(
            "Choose published Questions, confirm their order, then return to this local draft.",
          );
        }}
      >
        Choose published Questions
      </button>
      <div class="curriculum-save-actions">
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
  );
}
