// reusable_curriculum_create_dialog.tsx - local draft gate before a live curriculum creation.

import { Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { useNavigate } from "@solidjs/router";

import type { AlphaCourseDefinitionInput } from "../../../generated/api/AlphaCourseDefinitionInput";
import type { BlueprintDefinitionInput } from "../../../generated/api/BlueprintDefinitionInput";
import type { ReusableAssignmentDefinitionInput } from "../../../generated/api/ReusableAssignmentDefinitionInput";
import type { ReusableCurriculumClient } from "../../api/reusable_curriculum";
import {
  ProblemPicker,
  type ProblemPickerSelection,
  type ProblemPickerSource,
  type ProblemPickerSourceRepository,
} from "../problem_picker";
import { createAlphaWhenReady, createBlueprintWhenReady } from "./reusable_curriculum_creation";
import {
  alphaProblemPickerSources,
  appendPickedFixedEntries,
  emptyReusableDefinition,
} from "./reusable_curriculum_model";

export interface CurriculumCreateDialogProps {
  readonly kind: "blueprint" | "alpha";
  readonly client: ReusableCurriculumClient;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<ProblemPickerSource>;
  readonly onClose: () => void;
  readonly onFailure: (text: string) => void;
}

function detailPath(reference: string): string {
  return `/curriculum/${encodeURIComponent(reference)}`;
}

/** Keeps an incomplete create draft in the browser until it has meaningful reusable content. */
export function CurriculumCreateDialog(props: CurriculumCreateDialogProps): JSX.Element {
  const navigate = useNavigate();
  const initialDefinition = (): ReusableAssignmentDefinitionInput =>
    emptyReusableDefinition(
      props.kind === "blueprint" ? "Untitled reusable assignment" : "Untitled Alpha assignment",
    );
  const [title, setTitle] = createSignal(
    props.kind === "blueprint" ? "Untitled reusable assignment" : "Untitled Alpha curriculum",
  );
  const [definition, setDefinition] = createSignal(initialDefinition());
  const [showPicker, setShowPicker] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal(
    "Name this curriculum, then choose its first published question to make a complete reusable definition.",
  );
  let dialog!: HTMLDialogElement;
  let titleInput!: HTMLInputElement;
  let pickerTrigger: HTMLButtonElement | undefined;

  function closeDraft(): void {
    if (dialog.open) dialog.close();
    props.onClose();
  }

  function pickerSources(): ReadonlyArray<ProblemPickerSource> {
    return props.kind === "alpha" ? alphaProblemPickerSources() : props.pickerSources;
  }

  function chooseQuestions(selection: ProblemPickerSelection): void {
    setDefinition((current) => appendPickedFixedEntries(current, selection));
    setShowPicker(false);
    setMessage(
      `Added ${selection.questionIds.length} selected question${selection.questionIds.length === 1 ? "" : "s"}. Review the draft, then create the live curriculum.`,
    );
  }

  function blueprintDraft(): BlueprintDefinitionInput {
    return { definition: { ...definition(), title: title() } };
  }

  function alphaDraft(): AlphaCourseDefinitionInput {
    const reusableDefinition = { ...definition(), title: "Module 1 assignment" };
    return { title: title(), modules: [{ label: "Module 1", definitions: [reusableDefinition] }] };
  }

  async function save(): Promise<void> {
    setBusy(true);
    try {
      if (props.kind === "blueprint") {
        const result = await createBlueprintWhenReady(props.client, blueprintDraft());
        if (result.kind === "invalid") {
          setMessage(result.message);
          return;
        }
        navigate(detailPath(result.value.blueprint.reference));
      } else {
        const result = await createAlphaWhenReady(props.client, alphaDraft());
        if (result.kind === "invalid") {
          setMessage(result.message);
          return;
        }
        navigate(detailPath(result.value.alpha.reference));
      }
    } catch (error: unknown) {
      const text =
        error instanceof Error
          ? error.message
          : "The curriculum could not be created. This local draft remains ready to retry.";
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
          <h2 id="curriculum-create-heading">
            Create {props.kind === "blueprint" ? "a blueprint" : "an Alpha curriculum"}
          </h2>
          <p>
            {props.kind === "alpha"
              ? "Public Alpha drafts use public-library questions so approved instructors can reuse every entry."
              : "Drafts stay here until they include a reusable assignment with published questions."}
          </p>
        </div>
        <button type="button" class="quiet-action" disabled={busy()} onClick={closeDraft}>
          Close draft
        </button>
      </div>
      <p class="curriculum-notice" role="status">
        {message()}
      </p>
      <label>
        {props.kind === "blueprint" ? "Assignment title" : "Curriculum title"}
        <input
          ref={(element) => {
            titleInput = element;
          }}
          value={title()}
          maxlength="200"
          onInput={(event) => {
            setTitle(event.currentTarget.value);
            setMessage(
              "Title updated. Choose published questions to complete this live curriculum.",
            );
          }}
        />
      </label>
      <p>
        {definition().entries.length === 0
          ? "No questions selected yet."
          : `${definition().entries.length} fixed question${definition().entries.length === 1 ? "" : "s"} selected in order.`}
      </p>
      <button
        type="button"
        disabled={busy()}
        onClick={(event) => {
          pickerTrigger = event.currentTarget;
          setShowPicker(true);
          setMessage(
            "Choose published questions, confirm their order, then return to this create draft.",
          );
        }}
      >
        Choose published questions
      </button>
      <div class="curriculum-save-actions">
        <button type="button" disabled={busy()} onClick={() => void save()}>
          {busy() ? "Creating..." : "Create live curriculum"}
        </button>
      </div>
      <Show when={showPicker()}>
        <ProblemPicker
          repository={props.pickerRepository}
          sources={pickerSources()}
          mode="many"
          maximumSelection={1024}
          trigger={pickerTrigger}
          title="Choose the first reusable questions"
          confirmLabel="Use selected questions"
          onConfirm={chooseQuestions}
          onCancel={() => setShowPicker(false)}
        />
      </Show>
    </dialog>
  );
}
