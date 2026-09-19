// blueprint_pool_members_editor.tsx - local member draft for a retained Blueprint-owned Pool.

import { For, Show, createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { BlueprintAssessmentEntryInput } from "../../../generated/api/BlueprintAssessmentEntryInput";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { MAX_REUSABLE_ENTRIES } from "./blueprint_course_model";
import {
  QuestionPicker,
  type QuestionPickerSelection,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../question_picker";

type PoolEntry = Extract<BlueprintAssessmentEntryInput, { kind: "pool" }>;

export interface BlueprintPoolMembersEditorProps {
  readonly entry: PoolEntry;
  readonly blueprintCourseId: string;
  readonly assessmentId: string;
  readonly client: BlueprintCourseClient;
  readonly editable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onChange: (pool: PoolEntry["pool"], message: string) => void;
  readonly onClose: () => void;
  readonly onInvalidDraftChange: (invalid: boolean) => void;
}

/** Only exact, scoped reads occur here; all writes belong to the parent Blueprint Save. */
export function BlueprintPoolMembersEditor(props: BlueprintPoolMembersEditorProps): JSX.Element {
  const initialPool = structuredClone(props.entry.pool);
  const questionPoolId = initialPool.question_pool_id;
  const questionPoolEditNumber = initialPool.question_pool_edit_number;
  const [members, setMembers] = createSignal<QuestionRevisionTuple[]>();
  const [error, setError] = createSignal("");
  const [pickerOpen, setPickerOpen] = createSignal(false);
  let trigger: HTMLButtonElement | undefined;
  let disposed = false;
  onCleanup(() => {
    disposed = true;
    props.onInvalidDraftChange(false);
  });

  createEffect(() => {
    const current = props.entry.pool;
    if (
      current.question_pool_id !== questionPoolId ||
      current.question_pool_edit_number !== questionPoolEditNumber
    )
      props.onClose();
  });

  async function loadMembers(): Promise<void> {
    try {
      const result = await props.client.getBlueprintPoolMembers(
        props.blueprintCourseId,
        props.assessmentId,
        questionPoolId,
      );
      if (disposed) return;
      // ASVS 2.2.1: reject stale Pool membership rather than silently choosing another Pool.
      if (
        result.questionPoolId !== questionPoolId ||
        result.questionPoolEditNumber !== questionPoolEditNumber
      ) {
        setError(
          "Pool membership has changed. Cancel this editor and refresh the Blueprint Course before editing members.",
        );
        return;
      }
      setMembers(
        initialPool.kind === "retained" && initialPool.members !== null
          ? initialPool.members
          : result.members,
      );
    } catch {
      // ASVS 16.5.1: never expose transport internals or answer-bearing error payloads.
      if (!disposed)
        setError(
          "Pool members could not be loaded. Cancel and try again; your Blueprint draft is unchanged.",
        );
    }
  }
  onMount(() => {
    void loadMembers();
  });

  function changeMembers(next: QuestionRevisionTuple[], attested = false): void {
    setError("");
    setMembers(next);
    props.onChange(
      {
        kind: "retained",
        question_pool_id: questionPoolId,
        question_pool_edit_number: questionPoolEditNumber,
        members: next,
        interchangeabilityAttested: attested,
      },
      "Pool members updated locally. Review interchangeability, then Save the Blueprint Course to retain these changes.",
    );
    props.onInvalidDraftChange(
      next.length === 0 || next.length < props.entry.selection_count || !attested,
    );
  }

  function addQuestions(selection: QuestionPickerSelection): void {
    const next = [...(members() ?? [])];
    for (const question of selection.questions) {
      const questionRevisionTuple = question.row.questionRevisionTuple;
      if (!questionRevisionTuple) {
        setError(
          "A selected Question has no exact Revision Tuple. No members were added; refresh the Question Picker and try again.",
        );
        setPickerOpen(false);
        return;
      }
      if (!next.some((member) => member.questionId === questionRevisionTuple.questionId))
        next.push(questionRevisionTuple);
    }
    setPickerOpen(false);
    changeMembers(next);
  }

  function move(index: number, offset: -1 | 1): void {
    const next = [...(members() ?? [])];
    const member = next[index];
    const adjacent = next[index + offset];
    if (!member || !adjacent) return;
    next[index] = adjacent;
    next[index + offset] = member;
    changeMembers(next);
  }

  return (
    <section class="blueprint-course-content-card" aria-label="Blueprint Pool members">
      <h4>
        Question Pool {questionPoolId}, Edit {questionPoolEditNumber}
      </h4>
      <p>
        Members are exact Question Revision Tuples, in authored order. This edit changes only this
        Blueprint Assessment-owned Pool, not its source or adopted Course Instances.
      </p>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
      <Show
        when={members()}
        fallback={
          <Show when={!error()}>
            <p role="status">Loading Pool members...</p>
          </Show>
        }
      >
        {(list) => (
          <>
            <ol class="blueprint-course-entry-list">
              <For each={list()}>
                {(member, index) => (
                  <li>
                    {/* ASVS 1.2.1: ordinary JSX text keeps IDs inert; no HTML or raw JSON rendering. */}
                    <span>
                      Question {member.questionId}, Revision {member.revisionNumber}
                    </span>
                    <Show when={props.editable}>
                      <div class="blueprint-course-inline-actions">
                        <button
                          type="button"
                          disabled={index() === 0}
                          onClick={() => move(index(), -1)}
                        >
                          Move earlier
                        </button>
                        <button
                          type="button"
                          disabled={index() === list().length - 1}
                          onClick={() => move(index(), 1)}
                        >
                          Move later
                        </button>
                        <button
                          type="button"
                          onClick={() =>
                            changeMembers(list().filter((_, position) => position !== index()))
                          }
                        >
                          Remove member
                        </button>
                      </div>
                    </Show>
                  </li>
                )}
              </For>
            </ol>
            <Show when={props.editable}>
              <button
                type="button"
                disabled={list().length >= MAX_REUSABLE_ENTRIES}
                onClick={(event) => {
                  trigger = event.currentTarget;
                  setPickerOpen(true);
                }}
              >
                Add Pool members
              </button>
              <Show
                when={props.entry.pool.kind === "retained" && props.entry.pool.members !== null}
              >
                <label>
                  <input
                    type="checkbox"
                    checked={
                      props.entry.pool.kind === "retained" &&
                      props.entry.pool.interchangeabilityAttested
                    }
                    onChange={(event) => changeMembers(list(), event.currentTarget.checked)}
                  />
                  I attest that these Questions are interchangeable for this Pool's learning
                  objective and scoring.
                </label>
                <Show when={list().length < props.entry.selection_count || list().length === 0}>
                  <p role="alert">
                    Keep at least one member and enough members for the Pool selection count.
                  </p>
                </Show>
                <Show
                  when={
                    props.entry.pool.kind === "retained" &&
                    !props.entry.pool.interchangeabilityAttested
                  }
                >
                  <p role="alert">Attest interchangeability before saving changed Pool members.</p>
                </Show>
              </Show>
            </Show>
          </>
        )}
      </Show>
      <div class="blueprint-course-inline-actions">
        <button
          type="button"
          onClick={() => {
            if (props.editable)
              props.onChange(
                initialPool,
                "Pool member edits canceled. Other Blueprint draft changes are preserved.",
              );
            props.onClose();
          }}
        >
          {props.editable ? "Cancel member edits" : "Close Pool members"}
        </button>
        <Show when={props.editable && members()}>
          <button type="button" onClick={props.onClose}>
            Done editing members
          </button>
        </Show>
      </div>
      <Show when={pickerOpen()}>
        <QuestionPicker
          repository={props.pickerRepository}
          sources={props.pickerSources}
          mode="many"
          maximumSelection={Math.max(1, MAX_REUSABLE_ENTRIES - (members()?.length ?? 0))}
          trigger={trigger}
          title="Add Blueprint Pool members"
          confirmLabel="Add exact Question Revisions"
          onConfirm={addQuestions}
          onCancel={() => setPickerOpen(false)}
        />
      </Show>
    </section>
  );
}
