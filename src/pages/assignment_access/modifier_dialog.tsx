// modifier_dialog.tsx - accessible editor for one M2, M3, or M4 modifier.

import { For, Match, Show, Switch, createEffect, createSignal, type JSX } from "solid-js";

import type { CourseGroupSummaryView } from "../../../generated/api/CourseGroupSummaryView";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import {
  emptyPatchDraft,
  eligibleModifierGroups,
  policyRequest,
  scheduleOffsetRequest,
  type ModifierMode,
  type ModifierPatchDraft,
  type ModifierScope,
  type PreviewSubject,
} from "./model";

export interface ModifierDialogProps {
  readonly scope: ModifierScope;
  readonly groups: ReadonlyArray<CourseGroupSummaryView>;
  readonly subjects: ReadonlyArray<PreviewSubject>;
  readonly revision: TeachingOperationRevision;
  readonly busy: boolean;
  readonly revisionConflict?: boolean;
  readonly onClose: () => void;
  readonly onReloadLatestRevision?: () => void;
  readonly onSaveOffset: (group: CourseGroupSummaryView, value: string) => Promise<boolean>;
  readonly onSavePolicy: (
    scope: "groupAccommodation" | "individualException",
    target: string,
    mode: ModifierMode,
    draft: ModifierPatchDraft,
  ) => Promise<boolean>;
  readonly onDelete: (scope: ModifierScope, target: string, name: string) => Promise<boolean>;
}

function scopeTitle(scope: ModifierScope): string {
  if (scope === "scheduleOffset") return "Group schedule offset";
  if (scope === "groupAccommodation") return "Group accommodation";
  return "Individual exception";
}

function modeCopy(mode: ModifierMode): string {
  return mode === "extendOnly" ? "Extend only" : "Override";
}

function updatePatch(
  draft: ModifierPatchDraft,
  field: keyof ModifierPatchDraft,
  next: Partial<ModifierPatchDraft[typeof field]>,
): ModifierPatchDraft {
  const result = { ...draft, [field]: { ...draft[field], ...next } };
  return result;
}

interface PatchFieldProps {
  readonly label: string;
  readonly field: keyof ModifierPatchDraft;
  readonly draft: () => ModifierPatchDraft;
  readonly setDraft: (next: ModifierPatchDraft) => void;
  readonly inputType: "datetime-local" | "number";
}

function PatchField(props: PatchFieldProps): JSX.Element {
  const field = (): ModifierPatchDraft[typeof props.field] => props.draft()[props.field];
  return (
    <fieldset class="assignment-access-patch-field">
      <legend>{props.label}</legend>
      <label>
        <input
          type="radio"
          name={`${props.field}-kind`}
          checked={field().kind === "inherit"}
          onChange={() =>
            props.setDraft(updatePatch(props.draft(), props.field, { kind: "inherit" }))
          }
        />{" "}
        Inherit
      </label>
      <label>
        <input
          type="radio"
          name={`${props.field}-kind`}
          checked={field().kind === "set"}
          onChange={() => props.setDraft(updatePatch(props.draft(), props.field, { kind: "set" }))}
        />{" "}
        Set
      </label>
      <label>
        <input
          type="radio"
          name={`${props.field}-kind`}
          checked={field().kind === "unrestricted"}
          onChange={() =>
            props.setDraft(updatePatch(props.draft(), props.field, { kind: "unrestricted" }))
          }
        />{" "}
        Unrestricted
      </label>
      <Show when={field().kind === "set"}>
        <input
          type={props.inputType}
          min={props.inputType === "number" ? "1" : undefined}
          value={field().value}
          onInput={(event) =>
            props.setDraft(
              updatePatch(props.draft(), props.field, { value: event.currentTarget.value }),
            )
          }
        />
      </Show>
    </fieldset>
  );
}

export function ModifierDialog(props: ModifierDialogProps): JSX.Element {
  const eligibleGroups = (): ReadonlyArray<CourseGroupSummaryView> =>
    eligibleModifierGroups(props.scope, props.groups);
  const [selectedGroup, setSelectedGroup] = createSignal<CourseGroupSummaryView | undefined>(
    eligibleGroups()[0],
  );
  const [selectedSubject, setSelectedSubject] = createSignal<PreviewSubject | undefined>(
    props.subjects[0],
  );
  const [offsetSeconds, setOffsetSeconds] = createSignal("");
  const [mode, setMode] = createSignal<ModifierMode>("extendOnly");
  const [draft, setDraft] = createSignal(emptyPatchDraft());
  const [error, setError] = createSignal("");
  let heading: HTMLHeadingElement | undefined;
  createEffect(() => queueMicrotask(() => heading?.focus()));

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setError("");
    try {
      if (props.scope === "scheduleOffset") {
        const group = selectedGroup();
        if (group === undefined) throw new Error("Choose a course group.");
        scheduleOffsetRequest(offsetSeconds());
        if (!(await props.onSaveOffset(group, offsetSeconds()))) return;
      } else if (props.scope === "groupAccommodation") {
        const group = selectedGroup();
        if (group === undefined) throw new Error("Choose a course group.");
        policyRequest(mode(), draft());
        if (!(await props.onSavePolicy("groupAccommodation", group.reference, mode(), draft())))
          return;
      } else {
        const subject = selectedSubject();
        if (subject === undefined) throw new Error("Choose a learner.");
        policyRequest(mode(), draft());
        if (!(await props.onSavePolicy("individualException", subject.reference, mode(), draft())))
          return;
      }
    } catch (caught: unknown) {
      const message = caught instanceof Error ? caught.message : "The modifier could not be saved.";
      setError(
        message.startsWith("API request")
          ? "The modifier could not be saved. Your draft is retained."
          : message,
      );
    }
  }

  async function remove(): Promise<void> {
    const group = selectedGroup();
    const subject = selectedSubject();
    const target = props.scope === "individualException" ? subject?.reference : group?.reference;
    const name = props.scope === "individualException" ? subject?.display : group?.title;
    if (target === undefined || name === undefined) {
      setError("Choose a named target before removing a modifier.");
      return;
    }
    try {
      if (!(await props.onDelete(props.scope, target, name))) {
        setError("The modifier could not be removed. Your draft is retained.");
      }
    } catch {
      setError("The modifier could not be removed. Your draft is retained.");
    }
  }

  return (
    <div class="assignment-access-dialog-backdrop" role="presentation">
      <dialog
        class="assignment-access-dialog"
        aria-labelledby="assignment-access-dialog-heading"
        ref={(element) => queueMicrotask(() => element.showModal())}
        onCancel={(event) => {
          event.preventDefault();
          props.onClose();
        }}
      >
        <h2
          ref={(element) => (heading = element)}
          id="assignment-access-dialog-heading"
          tabindex="-1"
        >
          {scopeTitle(props.scope)}
        </h2>
        <form onSubmit={(event) => void submit(event)}>
          <Switch>
            <Match when={props.scope !== "individualException"}>
              <label class="assignment-access-field">
                Course group
                <select
                  disabled={props.busy}
                  onChange={(event) =>
                    setSelectedGroup(
                      eligibleGroups().find(
                        (group) => group.reference === event.currentTarget.value,
                      ),
                    )
                  }
                >
                  <For
                    each={eligibleGroups()}
                    fallback={<option value="">No eligible course groups are available</option>}
                  >
                    {(group) => <option value={group.reference}>{group.title}</option>}
                  </For>
                </select>
              </label>
            </Match>
            <Match when={props.scope === "individualException"}>
              <label class="assignment-access-field">
                Learner
                <select
                  disabled={props.busy}
                  onChange={(event) =>
                    setSelectedSubject(
                      props.subjects.find(
                        (subject) => subject.reference === event.currentTarget.value,
                      ),
                    )
                  }
                >
                  <For
                    each={props.subjects}
                    fallback={<option value="">No authorized learner list is available</option>}
                  >
                    {(subject) => <option value={subject.reference}>{subject.display}</option>}
                  </For>
                </select>
              </label>
            </Match>
          </Switch>
          <Show when={props.scope === "scheduleOffset"}>
            <label class="assignment-access-field">
              Offset in seconds{" "}
              <input
                type="number"
                required
                step="1"
                value={offsetSeconds()}
                onInput={(event) => setOffsetSeconds(event.currentTarget.value)}
              />
            </label>
          </Show>
          <Show when={props.scope !== "scheduleOffset"}>
            <fieldset class="assignment-access-mode">
              <legend>Modifier semantics</legend>
              <For each={["extendOnly", "override"] as const}>
                {(choice) => (
                  <label>
                    <input
                      type="radio"
                      name="modifier-mode"
                      checked={mode() === choice}
                      onChange={() => setMode(choice)}
                    />{" "}
                    {modeCopy(choice)}
                  </label>
                )}
              </For>
            </fieldset>
            <p class="assignment-access-help">
              Inherit leaves the current resolved value unchanged. Set applies a value. Unrestricted
              removes this limit or time boundary. Extend only cannot make access more restrictive.
            </p>
            <PatchField
              label="Available"
              field="availableAt"
              draft={draft}
              setDraft={setDraft}
              inputType="datetime-local"
            />
            <PatchField
              label="Due"
              field="dueAt"
              draft={draft}
              setDraft={setDraft}
              inputType="datetime-local"
            />
            <PatchField
              label="Closes"
              field="closesAt"
              draft={draft}
              setDraft={setDraft}
              inputType="datetime-local"
            />
            <PatchField
              label="Whole-run seconds"
              field="timeLimitSeconds"
              draft={draft}
              setDraft={setDraft}
              inputType="number"
            />
            <PatchField
              label="Attempt limit"
              field="attemptLimit"
              draft={draft}
              setDraft={setDraft}
              inputType="number"
            />
          </Show>
          <Show when={error()}>
            <p class="assignment-access-error" role="alert">
              {error()}
            </p>
          </Show>
          <Show when={props.revisionConflict && props.onReloadLatestRevision !== undefined}>
            <p class="assignment-access-help">
              This assignment changed elsewhere. Reload its latest revision before retrying; your
              modifier draft remains open.
            </p>
            <button type="button" disabled={props.busy} onClick={props.onReloadLatestRevision}>
              Reload latest assignment revision
            </button>
          </Show>
          <div class="assignment-access-actions">
            <button type="button" disabled={props.busy} onClick={props.onClose}>
              Cancel
            </button>
            <button type="button" disabled={props.busy} onClick={() => void remove()}>
              Remove named modifier
            </button>
            <button type="submit" disabled={props.busy}>
              {props.busy ? "Saving..." : "Save modifier"}
            </button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
