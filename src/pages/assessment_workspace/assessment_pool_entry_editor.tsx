// Exact-member editor for one Assessment-owned Question Pool fork.

import { For, Show, createMemo, createSignal, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { AssessmentQuestionPickerEntry } from "../../api/assessment_release";
import { questionRevisionKey } from "./assessment_workspace_questions_model";
import { CourseClassificationSummary } from "../../components/course_classification_summary";

export interface AssessmentPoolEntryEditorProps {
  readonly entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>;
  readonly fork: AssessmentQuestionPoolForkView | undefined;
  readonly exactMembersUnavailable: boolean;
  readonly availableQuestions: ReadonlyArray<AssessmentQuestionPickerEntry>;
  readonly mutationsEnabled: boolean;
  readonly busy: boolean;
  readonly onSelectionCount: (selectionCount: number) => void;
  readonly onReplaceMembers: (members: ReadonlyArray<QuestionRevisionReference>) => void;
}

/** Renders exact immutable fork members and only the two permitted Assessment-owned mutations. */
export function AssessmentPoolEntryEditor(props: AssessmentPoolEntryEditorProps): JSX.Element {
  const [candidateKey, setCandidateKey] = createSignal("");
  const [attested, setAttested] = createSignal(false);
  const candidates = createMemo(() => {
    const existing = new Set(
      props.fork?.members.map((member) => questionRevisionKey(member.questionRevision)) ?? [],
    );
    return props.availableQuestions.filter(
      (candidate) => !existing.has(questionRevisionKey(candidate.reference)),
    );
  });
  const selectedCandidate = createMemo(() =>
    candidates().find((candidate) => questionRevisionKey(candidate.reference) === candidateKey()),
  );

  function submitSelectionCount(value: string): void {
    const selectionCount = Number(value);
    if (
      !Number.isSafeInteger(selectionCount) ||
      selectionCount < 1 ||
      props.fork === undefined ||
      selectionCount > props.fork.members.length
    ) {
      return;
    }
    if (selectionCount !== props.entry.selectionCount) props.onSelectionCount(selectionCount);
  }

  function append(): void {
    const candidate = selectedCandidate();
    if (candidate === undefined || !attested() || !props.mutationsEnabled || props.busy) return;
    props.onReplaceMembers([
      ...(props.fork?.members.map((member) => member.questionRevision) ?? []),
      candidate.reference,
    ]);
    setCandidateKey("");
    setAttested(false);
  }

  function replaceMembers(members: ReadonlyArray<QuestionRevisionReference>): void {
    if (!attested() || !props.mutationsEnabled || props.busy) return;
    props.onReplaceMembers(members);
    setAttested(false);
  }

  return (
    <section class="assessment-pool-fork" aria-labelledby={`assessment-pool-${props.entry.id}`}>
      <h3 id={`assessment-pool-${props.entry.id}`}>
        Question Pool {props.entry.questionPoolRevision.questionPoolId} Revision{" "}
        {props.entry.questionPoolRevision.revisionNumber}
      </h3>
      <Show
        when={props.fork}
        fallback={
          <p class="assessment-editor-note">
            {props.exactMembersUnavailable
              ? "This Assessment's exact pinned Pool members could not load. Reload the Assessment and try again."
              : "Loading this Assessment's exact pinned Pool members..."}
          </p>
        }
      >
        {(fork) => (
          <>
            <h4>{fork().metadata.title}</h4>
            <p>{fork().metadata.description}</p>
            <CourseClassificationSummary value={fork().metadata} />
            <p>
              This Assessment owns this imported Pool fork. It selects {props.entry.selectionCount}{" "}
              of {fork().members.length} exact pinned Questions.
            </p>
            <label>
              <input
                type="checkbox"
                checked={attested()}
                onChange={(event) => setAttested(event.currentTarget.checked)}
              />
              These Questions are interchangeable assessments of the intended learning.
            </label>
            <ol>
              <For each={fork().members}>
                {(member, index) => (
                  <li>
                    <strong>{member.questionRevision.questionId}</strong> Revision{" "}
                    {member.questionRevision.revisionNumber}:{" "}
                    {member.question.question_library.summary.metadata.questionTitle}
                    <fieldset disabled={!props.mutationsEnabled || props.busy || !attested()}>
                      <legend>Membership order</legend>
                      <button
                        type="button"
                        disabled={index() === 0}
                        onClick={() => {
                          const members = fork().members.map((current) => current.questionRevision);
                          [members[index() - 1], members[index()]] = [
                            members[index()]!,
                            members[index() - 1]!,
                          ];
                          replaceMembers(members);
                        }}
                      >
                        Move earlier
                      </button>
                      <button
                        type="button"
                        disabled={index() === fork().members.length - 1}
                        onClick={() => {
                          const members = fork().members.map((current) => current.questionRevision);
                          [members[index()], members[index() + 1]] = [
                            members[index() + 1]!,
                            members[index()]!,
                          ];
                          replaceMembers(members);
                        }}
                      >
                        Move later
                      </button>
                      <button
                        type="button"
                        disabled={fork().members.length <= props.entry.selectionCount}
                        onClick={() =>
                          replaceMembers(
                            fork()
                              .members.filter((_current, currentIndex) => currentIndex !== index())
                              .map((current) => current.questionRevision),
                          )
                        }
                      >
                        Remove
                      </button>
                    </fieldset>
                  </li>
                )}
              </For>
            </ol>
            <Show when={fork().members.length <= props.entry.selectionCount}>
              <p class="assessment-editor-note">
                Lower the selection count before removing a Pool member.
              </p>
            </Show>
            <fieldset disabled={!props.mutationsEnabled || props.busy}>
              <legend>Questions selected for each Attempt</legend>
              <label class="assessment-editor-field">
                Selection count
                <input
                  type="number"
                  min="1"
                  max={fork().members.length}
                  value={props.entry.selectionCount}
                  onChange={(event) => submitSelectionCount(event.currentTarget.value)}
                />
              </label>
              <p class="assessment-editor-note">
                This changes only this Assessment-owned Pool entry.
              </p>
            </fieldset>
            <fieldset disabled={!props.mutationsEnabled || props.busy || candidates().length === 0}>
              <legend>Add one pinned Question</legend>
              <label class="assessment-editor-field">
                Available published Question
                <select
                  value={candidateKey()}
                  onChange={(event) => setCandidateKey(event.currentTarget.value)}
                >
                  <option value="">Choose a Question</option>
                  <For each={candidates()}>
                    {(candidate) => (
                      <option value={questionRevisionKey(candidate.reference)}>
                        {candidate.reference.questionId} Revision{" "}
                        {candidate.reference.revisionNumber}: {candidate.description}
                      </option>
                    )}
                  </For>
                </select>
              </label>
              <button
                type="button"
                disabled={selectedCandidate() === undefined || !attested()}
                onClick={append}
              >
                Add pinned Question
              </button>
            </fieldset>
          </>
        )}
      </Show>
      <Show when={!props.mutationsEnabled}>
        <p class="assessment-editor-note">
          Save or reload the pending Assessment changes before changing this Pool.
        </p>
      </Show>
    </section>
  );
}
