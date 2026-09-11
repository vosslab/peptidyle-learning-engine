import { A } from "@solidjs/router";
import { For, Show, createSignal, type JSX } from "solid-js";

import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type {
  AssignmentReleaseValidation,
  SaveLiveAssignmentInput,
} from "../../api/assignment_release";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssignmentWorkspaceConflictError } from "../../api/http_client/assignment_release";
import { assignmentWorkspacePath } from "./assignment_workspace_paths";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import {
  canonicalLocalDateAndTime,
  dueDateDraft,
  dueTimeDraft,
  localDueDateAndTime,
} from "./assignment_workspace_policy_model";

const FEEDBACK_FIELDS = [
  ["score", "Score"],
  ["per_item_correctness", "Per-item correctness"],
  [
    "submitted_response",
    "Previous-attempt response",
    "Controls the Student's recorded response in previous attempts. Never leaves correctness visible when Per-item correctness permits it.",
  ],
  ["question_feedback", "Question feedback"],
  ["question_answer", "Correct answer"],
  ["question_answer_explanation", "Question answer explanation"],
  [
    "class_statistics",
    "Class statistics",
    "Default: Never. Choose a later timing only when sharing class statistics is appropriate.",
  ],
] as const;

function inputFrom(workspace: SaveLiveAssignmentInput): SaveLiveAssignmentInput {
  return { ...workspace };
}

/** Edits the full direct resource while keeping timing and feedback controls independent. */
export function AssignmentWorkspacePoliciesPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assignment().workspace;
  const [instructions, setInstructions] = createSignal(initial.instructions);
  const [dueDate, setDueDate] = createSignal(dueDateDraft(initial.dueAt));
  const [dueTime, setDueTime] = createSignal(dueTimeDraft(initial.dueAt));
  const [timeLimit, setTimeLimit] = createSignal(
    initial.assignmentAttemptTimeLimitSeconds?.toString() ?? "",
  );
  const [attemptLimit, setAttemptLimit] = createSignal(initial.attemptLimit?.toString() ?? "");
  const [lateWorkRule, setLateWorkRule] = createSignal<LateWorkRule>(initial.lateWorkRule);
  const [activityRules, setActivityRules] = createSignal(initial.activityRules);
  const [feedbackRules, setFeedbackRules] = createSignal(initial.studentFeedbackReleaseRule);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [releaseValidation, setReleaseValidation] = createSignal<AssignmentReleaseValidation>();
  const [validationFailed, setValidationFailed] = createSignal(false);

  function integer(value: string): number | null | undefined {
    if (value === "") return null;
    return /^[1-9][0-9]*$/u.test(value) ? Number(value) : undefined;
  }
  function currentInput(): SaveLiveAssignmentInput | null {
    const base = workspace.assignment().workspace;
    const parsedDueAt = canonicalLocalDateAndTime(localDueDateAndTime(dueDate(), dueTime()));
    const parsedTimeLimit = integer(timeLimit());
    const parsedAttemptLimit = integer(attemptLimit());
    if (
      (dueDate() !== "" && parsedDueAt === null) ||
      parsedTimeLimit === undefined ||
      parsedAttemptLimit === undefined
    )
      return null;
    return {
      ...inputFrom({
        title: base.title,
        instructions: instructions(),
        questionIds: base.questions.map((question) => question.questionId),
        dueAt: parsedDueAt,
        lateWorkRule: lateWorkRule(),
        assignmentAttemptTimeLimitSeconds: parsedTimeLimit,
        attemptLimit: parsedAttemptLimit,
        activityRules: activityRules(),
        studentFeedbackReleaseRule: feedbackRules(),
      }),
    };
  }
  function updateOrder(shuffled: boolean): void {
    setActivityRules((current) => ({
      ...current,
      assignmentQuestionOrderRule: shuffled ? "shuffled" : "authoredOrder",
    }));
  }
  function updateFeedback(field: (typeof FEEDBACK_FIELDS)[number][0], value: string): void {
    if (
      value === "during_attempt" ||
      value === "after_submit" ||
      value === "after_due" ||
      value === "after_close" ||
      value === "never"
    ) {
      setFeedbackRules((current) => ({ ...current, [field]: value }));
    }
  }
  async function save(): Promise<void> {
    const input = currentInput();
    if (input === null) {
      setMessage(
        "Enter a complete local due time and positive whole-number limits, or leave them blank.",
      );
      return;
    }
    if (needsReload()) {
      setMessage("Reload the latest assignment before saving. Your typed policies remain here.");
      return;
    }
    setBusy(true);
    try {
      await workspace.save(input);
      setReleaseValidation(undefined);
      setMessage("Assignment policies saved. The current assignment now uses the new revision.");
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssignmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This assignment changed elsewhere. Reload latest assignment before saving; your typed policies remain here."
          : "Assignment policies were not saved. Try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function reload(): Promise<void> {
    setBusy(true);
    try {
      const latest = await workspace.reloadAssignment();
      const current = latest.workspace;
      setInstructions(current.instructions);
      setDueDate(dueDateDraft(current.dueAt));
      setDueTime(dueTimeDraft(current.dueAt));
      setTimeLimit(current.assignmentAttemptTimeLimitSeconds?.toString() ?? "");
      setAttemptLimit(current.attemptLimit?.toString() ?? "");
      setLateWorkRule(current.lateWorkRule);
      setActivityRules(current.activityRules);
      setFeedbackRules(current.studentFeedbackReleaseRule);
      setNeedsReload(false);
      setReleaseValidation(undefined);
      setMessage("Latest assignment loaded. Review the current policies.");
    } catch {
      setMessage("The latest assignment could not load. Your typed policies remain here.");
    } finally {
      setBusy(false);
    }
  }
  async function validateRelease(): Promise<void> {
    if (needsReload()) {
      setMessage("Reload the latest assignment before checking its release readiness.");
      return;
    }
    setBusy(true);
    setValidationFailed(false);
    setReleaseValidation(undefined);
    try {
      const validation = await applicationApi.client.validateLiveAssignmentRelease(
        workspace.courseReference,
        workspace.assignmentReference,
      );
      setReleaseValidation(validation);
      setMessage(
        validation.canRelease
          ? "Release readiness checked. This saved assignment is ready to release."
          : "Release readiness checked. Resolve the listed requirements before releasing.",
      );
    } catch {
      setReleaseValidation(undefined);
      setValidationFailed(true);
      setMessage("Release readiness could not be checked. Try again.");
    } finally {
      setBusy(false);
    }
  }
  async function release(): Promise<void> {
    if (needsReload()) {
      setMessage("Reload the latest assignment before releasing it.");
      return;
    }
    setBusy(true);
    try {
      const current = workspace.assignment();
      const released = await workspace.release(current.etag);
      await workspace.reloadAssignment();
      setReleaseValidation(undefined);
      setMessage(`Assignment released as revision ${released.revisionNumber}.`);
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssignmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This assignment changed elsewhere. Reload latest assignment before releasing it."
          : "The assignment could not be released. Review its Questions and policies, then try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  const questionsPath = assignmentWorkspacePath(
    workspace.courseReference,
    workspace.assignmentReference,
    "questions",
  );
  return (
    <section class="assignment-workspace-policies" aria-labelledby="assignment-policies-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-policies-heading">Policies</h1>
        <p class="page-lede">
          Times use your Instructor time zone: {workspace.assignment().workspace.displayTimeZone}.
        </p>
      </header>
      <Show when={message()}>
        {(value) => (
          <p
            class="assignment-workspace-save-message"
            role={validationFailed() ? "alert" : "status"}
          >
            {value()}
          </p>
        )}
      </Show>
      <fieldset class="assignment-workspace-policy-controls" disabled={busy()} aria-busy={busy()}>
        <legend>Assignment policies</legend>
        <section class="assignment-editor-policy-panel">
          <h2>Assignment and delivery</h2>
          <label class="assignment-editor-field">
            Student instructions
            <textarea
              rows="4"
              value={instructions()}
              onInput={(event) => setInstructions(event.currentTarget.value)}
            />
          </label>
          <div class="assignment-workspace-schedule" role="group" aria-label="Due date and time">
            <label class="assignment-editor-field">
              Due date ({workspace.assignment().workspace.displayTimeZone})
              <input
                type="date"
                value={dueDate()}
                onInput={(event) => setDueDate(event.currentTarget.value)}
              />
            </label>
            <label class="assignment-editor-field">
              Due time
              <input
                type="time"
                step="0.001"
                value={dueTime()}
                onInput={(event) => setDueTime(event.currentTarget.value)}
              />
            </label>
          </div>
          <label class="assignment-editor-field">
            Time limit in seconds
            <input
              type="number"
              min="1"
              value={timeLimit()}
              onInput={(event) => setTimeLimit(event.currentTarget.value)}
            />
          </label>
          <label class="assignment-editor-field">
            Attempt limit
            <input
              type="number"
              min="1"
              value={attemptLimit()}
              onInput={(event) => setAttemptLimit(event.currentTarget.value)}
            />
          </label>
          <p>
            The late-work rule controls whether Students may begin or save responses after the due
            time. Saved work remains available for the Student to submit.
          </p>
          <label class="assignment-editor-field">
            Late-work rule
            <select
              value={lateWorkRule()}
              onChange={(event) => setLateWorkRule(event.currentTarget.value as LateWorkRule)}
            >
              <option value="reject">Reject late work</option>
              <option value="mark_late">Accept and mark late</option>
              <option value="accept">Accept late work</option>
            </select>
          </label>
          <label class="assignment-workspace-choice">
            <input
              type="checkbox"
              checked={activityRules().assignmentQuestionOrderRule === "shuffled"}
              onChange={(event) => updateOrder(event.currentTarget.checked)}
            />
            <span>Randomize question order</span>
          </label>
          <p>
            Students see one Question at a time. Answer-choice order is configured on each Question.
          </p>
        </section>
        <section class="assignment-editor-policy-panel">
          <h2>Student feedback</h2>
          <For each={FEEDBACK_FIELDS}>
            {([field, label, help]) => (
              <label class="assignment-editor-field">
                {label}
                <select
                  value={feedbackRules()[field]}
                  aria-describedby={help === undefined ? undefined : `feedback-${field}-help`}
                  onChange={(event) => updateFeedback(field, event.currentTarget.value)}
                >
                  <option value="during_attempt">During attempt</option>
                  <option value="after_submit">After submit</option>
                  <option value="after_due">After due</option>
                  <option value="after_close">After close</option>
                  <option value="never">Never</option>
                </select>
                <Show when={help !== undefined}>
                  <small id={`feedback-${field}-help`}>{help}</small>
                </Show>
              </label>
            )}
          </For>
        </section>
        <p class="assignment-editor-actions">
          <button
            class="primary-action"
            type="button"
            disabled={needsReload()}
            onClick={() => void save()}
          >
            {busy() ? "Saving policies..." : "Save assignment policies"}
          </button>
          <Show when={workspace.assignment().workspace.status === "unreleased"}>
            <button type="button" disabled={needsReload()} onClick={() => void validateRelease()}>
              {busy() ? "Checking release readiness..." : "Check release readiness"}
            </button>
            <button
              class="primary-action"
              type="button"
              disabled={needsReload()}
              onClick={() => void release()}
            >
              Release assignment
            </button>
          </Show>
          <Show when={needsReload()}>
            <button type="button" onClick={() => void reload()}>
              Reload latest assignment
            </button>
          </Show>
          <A class="quiet-link" href={questionsPath}>
            Edit Questions
          </A>
          <A
            class="quiet-link"
            href={`${assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference)}/delivery-check`}
            target="_blank"
            rel="noopener"
          >
            Check assignment delivery
          </A>
        </p>
        <Show when={releaseValidation()}>
          {(validation) => (
            <section
              class="assignment-workspace-release-readiness"
              aria-labelledby="assignment-release-readiness-heading"
              role={validation().canRelease ? undefined : "alert"}
            >
              <h2 id="assignment-release-readiness-heading">
                {validation().canRelease ? "Ready to release" : "Release needs attention"}
              </h2>
              <Show
                when={!validation().canRelease}
                fallback={<p>All current release requirements are met.</p>}
              >
                <ul>
                  <For each={validation().issues}>
                    {(issue) => (
                      <li>
                        {issue === "noPublishedQuestions"
                          ? "Select and save at least one published Question."
                          : issue === "questionUnavailable"
                            ? "A selected Question is unavailable. Review and save the Questions list."
                            : "Enter a positive time limit in Assignment policies, save, then check release readiness again."}
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
              <Show when={!validation().canRelease}>
                <A class="quiet-link" href={questionsPath}>
                  Review assignment Questions
                </A>
              </Show>
            </section>
          )}
        </Show>
      </fieldset>
    </section>
  );
}
