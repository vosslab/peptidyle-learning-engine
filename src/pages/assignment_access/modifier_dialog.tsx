import { For, Show, createEffect, createSignal, type JSX } from "solid-js";

import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import {
  emptyPatchDraft,
  policyRequest,
  type ModifierMode,
  type ModifierPatchDraft,
  type SelectedStudent,
} from "./model";

export interface ModifierDialogProps {
  readonly subjects: ReadonlyArray<SelectedStudent>;
  readonly revision: TeachingOperationRevision;
  readonly busy: boolean;
  readonly revisionConflict?: boolean;
  readonly onClose: () => void;
  readonly onReloadLatestRevision?: () => void;
  readonly onSavePolicy: (
    target: string,
    mode: ModifierMode,
    draft: ModifierPatchDraft,
  ) => Promise<boolean>;
  readonly onDelete: (target: string, name: string) => Promise<boolean>;
}

export function ModifierDialog(props: ModifierDialogProps): JSX.Element {
  const [subject, setSubject] = createSignal<SelectedStudent | undefined>(props.subjects[0]);
  const [mode, setMode] = createSignal<ModifierMode>("extendOnly");
  const [draft] = createSignal(emptyPatchDraft());
  const [error, setError] = createSignal("");
  let heading: HTMLHeadingElement | undefined;
  createEffect(() => queueMicrotask(() => heading?.focus()));

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const selected = subject();
    if (selected === undefined) {
      setError("Choose a Student Membership.");
      return;
    }
    try {
      policyRequest(mode(), draft());
      if (!(await props.onSavePolicy(selected.reference, mode(), draft()))) {
        setError("The accommodation could not be saved. Your draft is retained.");
      }
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The accommodation could not be saved.");
    }
  }

  return (
    <div class="assignment-access-dialog-backdrop" role="presentation">
      <dialog
        class="assignment-access-dialog"
        aria-labelledby="assignment-access-dialog-heading"
        ref={(element) => queueMicrotask(() => element.showModal())}
        onCancel={(event) => { event.preventDefault(); props.onClose(); }}
      >
        <h2 ref={(element) => (heading = element)} id="assignment-access-dialog-heading" tabindex="-1">
          Student accommodation
        </h2>
        <form onSubmit={(event) => void save(event)}>
          <label class="assignment-access-field">
            Student Membership
            <select disabled={props.busy} onChange={(event) =>
              setSubject(props.subjects.find((item) => item.reference === event.currentTarget.value))
            }>
              <For each={props.subjects} fallback={<option value="">No authorized students are available</option>}>
                {(item) => <option value={item.reference}>{item.display}</option>}
              </For>
            </select>
          </label>
          <fieldset class="assignment-access-mode">
            <legend>Accommodation semantics</legend>
            <For each={["extendOnly", "replace"] as const}>
              {(choice) => <label><input type="radio" name="modifier-mode" checked={mode() === choice} onChange={() => setMode(choice)} /> {choice === "extendOnly" ? "Extend only" : "Replace"}</label>}
            </For>
          </fieldset>
          <p class="assignment-access-help">The selected Student Membership receives this direct accommodation. Detailed field editing remains available through the Assignment policy form.</p>
          <Show when={error()}><p class="assignment-access-error" role="alert">{error()}</p></Show>
          <div class="assignment-access-actions">
            <button type="button" disabled={props.busy} onClick={props.onClose}>Cancel</button>
            <button type="submit" disabled={props.busy || subject() === undefined}>Save accommodation</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
