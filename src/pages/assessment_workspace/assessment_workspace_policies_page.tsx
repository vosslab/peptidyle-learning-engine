import { A } from "@solidjs/router";
import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type {
  AssessmentUnreleaseImpact,
  AssessmentReleaseValidation,
  LiveAssessmentWorkspace,
  SaveBaseAssessmentPolicyInput,
} from "../../api/assessment_release";
import { useApplicationApi } from "../../api/application_api";
import { ApiRequestError } from "../../api/http_client/error";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import {
  canonicalLocalDateAndTime,
  dueDateDraft,
  dueTimeDraft,
  localDueDateAndTime,
} from "./assessment_workspace_policy_model";
import {
  allBaseAssessmentPolicyEditsPersisted,
  baseAssessmentPolicyDraftChanged,
  baseAssessmentPolicyReloaded,
  baseAssessmentPolicyRequest,
  baseAssessmentPolicyRetry,
  baseAssessmentPolicySaveError,
  baseAssessmentPolicySaveSucceeded,
  createBaseAssessmentPolicyAutosaveState,
} from "./base_assessment_policy_autosave_model";

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

function baseAssessmentPolicyInput(
  workspace: LiveAssessmentWorkspace,
): SaveBaseAssessmentPolicyInput {
  return {
    instructions: workspace.instructions,
    dueAt: workspace.dueAt,
    availableAt: workspace.availableAt,
    closesAt: workspace.closesAt,
    lateWorkRule: workspace.lateWorkRule,
    assessmentAttemptTimeLimitSeconds: workspace.assessmentAttemptTimeLimitSeconds,
    attemptLimit: workspace.attemptLimit,
    activityRules: workspace.activityRules,
    studentFeedbackReleaseRule: workspace.studentFeedbackReleaseRule,
  };
}

/** Edits the full direct resource while keeping timing and feedback controls independent. */
export function AssessmentWorkspacePoliciesPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assessment().workspace;
  const [instructions, setInstructions] = createSignal(initial.instructions);
  const [dueDate, setDueDate] = createSignal(dueDateDraft(initial.dueAt));
  const [dueTime, setDueTime] = createSignal(dueTimeDraft(initial.dueAt));
  const [timeLimit, setTimeLimit] = createSignal(
    initial.assessmentAttemptTimeLimitSeconds?.toString() ?? "",
  );
  const [attemptLimit, setAttemptLimit] = createSignal(initial.attemptLimit?.toString() ?? "");
  const [lateWorkRule, setLateWorkRule] = createSignal<LateWorkRule>(initial.lateWorkRule);
  const [activityRules, setActivityRules] = createSignal(initial.activityRules);
  const [feedbackRules, setFeedbackRules] = createSignal(initial.studentFeedbackReleaseRule);
  const [policyState, setPolicyState] = createSignal(
    createBaseAssessmentPolicyAutosaveState(baseAssessmentPolicyInput(initial)),
  );
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [releaseValidation, setReleaseValidation] = createSignal<AssessmentReleaseValidation>();
  const [validationFailed, setValidationFailed] = createSignal(false);
  const [unreleaseImpact, setUnreleaseImpact] = createSignal<AssessmentUnreleaseImpact>();
  const [confirmationTitle, setConfirmationTitle] = createSignal("");
  let instructionSaveTimer: number | undefined;
  let activeRequestSeq: number | undefined;

  function integer(value: string): number | null | undefined {
    if (value === "") return null;
    return /^[1-9][0-9]*$/u.test(value) ? Number(value) : undefined;
  }
  function currentInput(): SaveBaseAssessmentPolicyInput | null {
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
      instructions: instructions(),
      dueAt: parsedDueAt,
      lateWorkRule: lateWorkRule(),
      assessmentAttemptTimeLimitSeconds: parsedTimeLimit,
      attemptLimit: parsedAttemptLimit,
      activityRules: activityRules(),
      studentFeedbackReleaseRule: feedbackRules(),
      availableAt: workspace.assessment().workspace.availableAt,
      closesAt: workspace.assessment().workspace.closesAt,
    };
  }
  function updateOrder(shuffled: boolean): void {
    setActivityRules((current) => ({
      ...current,
      assessmentQuestionOrderRule: shuffled ? "shuffled" : "authoredOrder",
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
  function classifySaveError(error: unknown): "rejected" | "failed" | "conflict" {
    if (error instanceof LiveAssessmentWorkspaceConflictError) return "conflict";
    if (error instanceof ApiRequestError && error.status === 422) return "rejected";
    return "failed";
  }
  function startSave(state = policyState()): void {
    const request = baseAssessmentPolicyRequest(state);
    if (request === undefined || activeRequestSeq === request.seq) return;
    activeRequestSeq = request.seq;
    void (async (): Promise<void> => {
      try {
        const saved = await workspace.saveBaseAssessmentPolicy(request.input);
        setReleaseValidation(undefined);
        const next = baseAssessmentPolicySaveSucceeded(
          policyState(),
          request.seq,
          baseAssessmentPolicyInput(saved.workspace),
          currentInput() !== null,
        );
        setPolicyState(next);
        if (next.persistence === "saved") {
          setMessage("Assessment policies saved. Future Attempts use the current policy values.");
        }
        activeRequestSeq = undefined;
        startSave(next);
      } catch (error: unknown) {
        const persistence = classifySaveError(error);
        const next = baseAssessmentPolicySaveError(policyState(), request.seq, persistence);
        setPolicyState(next);
        if (persistence === "conflict") {
          setNeedsReload(true);
          setMessage(
            "This assessment changed elsewhere. Reload server state before saving; your typed policies remain here.",
          );
        } else if (persistence === "rejected") {
          setMessage("The server did not accept these policy values.");
        } else {
          setMessage("Assessment policies were not saved. Retry with the current values.");
        }
        activeRequestSeq = undefined;
      }
    })();
  }
  function recordDraft(requestNow = true): void {
    const input = currentInput();
    if (needsReload()) {
      setPolicyState((state) => ({
        ...state,
        draft: input ?? state.draft,
        draftSeq: state.draftSeq + 1,
        pending: undefined,
        persistence: input === null ? "invalid" : "conflict",
      }));
      return;
    }
    if (input === null) {
      setPolicyState((state) => baseAssessmentPolicyDraftChanged(state, state.draft, false));
      setMessage(
        "Enter a complete local due time and positive whole-number limits, or leave them blank.",
      );
      return;
    }
    const next = baseAssessmentPolicyDraftChanged(policyState(), input, true, requestNow);
    setPolicyState(next);
    if (requestNow) startSave(next);
  }
  function scheduleInstructionsSave(): void {
    if (instructionSaveTimer !== undefined) window.clearTimeout(instructionSaveTimer);
    recordDraft(false);
    instructionSaveTimer = window.setTimeout(() => {
      const next = baseAssessmentPolicyRetry(policyState(), currentInput() !== null);
      setPolicyState(next);
      startSave(next);
    }, 600);
  }
  function saveInstructionsNow(): void {
    if (instructionSaveTimer !== undefined) window.clearTimeout(instructionSaveTimer);
    const next = baseAssessmentPolicyRetry(policyState(), currentInput() !== null);
    setPolicyState(next);
    startSave(next);
  }
  function retry(): void {
    const next = baseAssessmentPolicyRetry(policyState(), currentInput() !== null);
    setPolicyState(next);
    startSave(next);
  }
  async function reload(): Promise<void> {
    setBusy(true);
    try {
      const latest = await workspace.reloadAssessment();
      const current = latest.workspace;
      setInstructions(current.instructions);
      setDueDate(dueDateDraft(current.dueAt));
      setDueTime(dueTimeDraft(current.dueAt));
      setTimeLimit(current.assessmentAttemptTimeLimitSeconds?.toString() ?? "");
      setAttemptLimit(current.attemptLimit?.toString() ?? "");
      setLateWorkRule(current.lateWorkRule);
      setActivityRules(current.activityRules);
      setFeedbackRules(current.studentFeedbackReleaseRule);
      setPolicyState((state) =>
        baseAssessmentPolicyReloaded(state, baseAssessmentPolicyInput(current)),
      );
      setNeedsReload(false);
      setReleaseValidation(undefined);
      setMessage("Latest assessment loaded. Review the current policies.");
    } catch {
      setMessage("The latest assessment could not load. Your typed policies remain here.");
    } finally {
      setBusy(false);
    }
  }
  async function validateRelease(): Promise<void> {
    if (needsReload()) {
      setMessage("Reload the latest assessment before checking its release readiness.");
      return;
    }
    setBusy(true);
    setValidationFailed(false);
    setReleaseValidation(undefined);
    try {
      const validation = await applicationApi.client.validateLiveAssessmentRelease(
        workspace.courseReference,
        workspace.assessmentReference,
      );
      setReleaseValidation(validation);
      setMessage(
        validation.canRelease
          ? "Release readiness checked. This saved assessment is ready to release."
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
      setMessage("Reload the latest assessment before releasing it.");
      return;
    }
    setBusy(true);
    try {
      const current = workspace.assessment();
      const released = await workspace.release(current.etag);
      await loadUnreleaseImpact();
      setReleaseValidation(undefined);
      setMessage(`Assessment released. Current edit number: ${released.workspace.editNumber}.`);
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssessmentWorkspaceConflictError;
      setNeedsReload(conflict);
      if (conflict) setPolicyState((state) => ({ ...state, persistence: "conflict" }));
      setMessage(
        conflict
          ? "This assessment changed elsewhere. Reload latest assessment before releasing it."
          : "The assessment could not be released. Review its Questions and policies, then try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function loadUnreleaseImpact(): Promise<void> {
    if (workspace.assessment().workspace.status !== "released") return;
    try {
      const impact = await applicationApi.client.getLiveAssessmentUnreleaseImpact(
        workspace.courseReference,
        workspace.assessmentReference,
      );
      setUnreleaseImpact(impact);
    } catch (error: unknown) {
      setUnreleaseImpact(undefined);
      if (error instanceof ApiRequestError && error.status === 404) return;
      setValidationFailed(true);
      setMessage("Unrelease impact could not be loaded. Try loading the current assessment again.");
    }
  }
  function unreleaseFailureMessage(error: unknown): string {
    if (!(error instanceof ApiRequestError))
      return "The assessment could not be unreleased. Try again.";
    if (error.status === 404) return "This assessment workspace is unavailable.";
    if (error.status === 409)
      return "This assessment is no longer released. Load the current assessment.";
    if (error.status === 422)
      return "Enter the current Assessment title exactly to confirm Unrelease.";
    return "The assessment could not be unreleased. Try again.";
  }
  async function unrelease(): Promise<void> {
    const impact = unreleaseImpact();
    if (impact === undefined) {
      setMessage("Load the current Unrelease impact before confirming this action.");
      return;
    }
    if (confirmationTitle() !== impact.confirmationTitle) {
      setValidationFailed(true);
      setMessage("Enter the current Assessment title exactly to confirm Unrelease.");
      return;
    }
    if (needsReload()) {
      setMessage("Reload the latest assessment before unreleasing it.");
      return;
    }
    setBusy(true);
    try {
      const result = await workspace.unrelease(confirmationTitle());
      setUnreleaseImpact(undefined);
      setConfirmationTitle("");
      setReleaseValidation(undefined);
      setValidationFailed(false);
      setMessage(
        `Assessment unreleased. Deleted ${result.deleted.attemptCount} Attempts, ${result.deleted.submissionCount} submissions, and ${result.deleted.gradeCount} grades.`,
      );
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssessmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setValidationFailed(true);
      setMessage(
        conflict
          ? "This assessment changed elsewhere. Reload latest assessment before unreleasing it."
          : unreleaseFailureMessage(error),
      );
    } finally {
      setBusy(false);
    }
  }
  onMount(() => void loadUnreleaseImpact());
  onCleanup(() => {
    if (instructionSaveTimer !== undefined) window.clearTimeout(instructionSaveTimer);
  });
  const questionsPath = assessmentWorkspacePath(
    workspace.courseReference,
    workspace.assessmentReference,
    "questions",
  );
  return (
    <section class="assessment-workspace-policies" aria-labelledby="assessment-policies-heading">
      <header class="assessment-workspace-header">
        <p class="eyebrow">Assessment workspace</p>
        <h1 id="assessment-policies-heading">Policies</h1>
        <p class="page-lede">
          Times use your Instructor time zone: {workspace.assessment().workspace.displayTimeZone}.
        </p>
      </header>
      <p class="assessment-workspace-save-message" role="status">
        {policyState().persistence === "saving"
          ? "Saving"
          : policyState().persistence === "saved"
            ? "Saved"
            : policyState().persistence === "invalid"
              ? "Invalid"
              : policyState().persistence === "rejected"
                ? "Not accepted"
                : policyState().persistence === "failed"
                  ? "Save failed"
                  : "Conflict"}
      </p>
      <Show when={message()}>
        {(value) => (
          <p
            class="assessment-workspace-save-message"
            role={validationFailed() ? "alert" : "status"}
          >
            {value()}
          </p>
        )}
      </Show>
      <fieldset
        class="assessment-workspace-policy-controls"
        disabled={busy()}
        aria-busy={busy() || policyState().persistence === "saving"}
      >
        <legend>Assessment policies</legend>
        <section class="assessment-editor-policy-panel">
          <h2>Assessment and delivery</h2>
          <label class="assessment-editor-field">
            Student instructions
            <textarea
              rows="4"
              value={instructions()}
              onInput={(event) => {
                setInstructions(event.currentTarget.value);
                scheduleInstructionsSave();
              }}
              onBlur={saveInstructionsNow}
            />
          </label>
          <div class="assessment-workspace-schedule" role="group" aria-label="Due date and time">
            <label class="assessment-editor-field">
              Due date ({workspace.assessment().workspace.displayTimeZone})
              <input
                type="date"
                value={dueDate()}
                onInput={(event) => {
                  setDueDate(event.currentTarget.value);
                  recordDraft();
                }}
              />
            </label>
            <label class="assessment-editor-field">
              Due time
              <input
                type="time"
                step="0.001"
                value={dueTime()}
                onInput={(event) => {
                  setDueTime(event.currentTarget.value);
                  recordDraft();
                }}
              />
            </label>
          </div>
          <label class="assessment-editor-field">
            Time limit in seconds
            <input
              type="number"
              min="1"
              value={timeLimit()}
              onInput={(event) => {
                setTimeLimit(event.currentTarget.value);
                recordDraft();
              }}
            />
          </label>
          <label class="assessment-editor-field">
            Attempt limit
            <input
              type="number"
              min="1"
              value={attemptLimit()}
              onInput={(event) => {
                setAttemptLimit(event.currentTarget.value);
                recordDraft();
              }}
            />
          </label>
          <p>
            The late-work rule controls whether Students may begin or save responses after the due
            time. Saved work remains available for the Student to submit.
          </p>
          <label class="assessment-editor-field">
            Late-work rule
            <select
              value={lateWorkRule()}
              onChange={(event) => {
                setLateWorkRule(event.currentTarget.value as LateWorkRule);
                recordDraft();
              }}
            >
              <option value="reject">Reject late work</option>
              <option value="mark_late">Accept and mark late</option>
              <option value="accept">Accept late work</option>
            </select>
          </label>
          <label class="assessment-workspace-choice">
            <input
              type="checkbox"
              checked={activityRules().assessmentQuestionOrderRule === "shuffled"}
              onChange={(event) => {
                updateOrder(event.currentTarget.checked);
                recordDraft();
              }}
            />
            <span>Randomize question order</span>
          </label>
          <p>
            Students see one Question at a time. Answer-choice order is configured on each Question.
          </p>
        </section>
        <section class="assessment-editor-policy-panel">
          <h2>Student feedback</h2>
          <For each={FEEDBACK_FIELDS}>
            {([field, label, help]) => (
              <label class="assessment-editor-field">
                {label}
                <select
                  value={feedbackRules()[field]}
                  aria-describedby={help === undefined ? undefined : `feedback-${field}-help`}
                  onChange={(event) => {
                    updateFeedback(field, event.currentTarget.value);
                    recordDraft();
                  }}
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
        <p class="assessment-editor-actions">
          <Show when={policyState().persistence === "failed"}>
            <button class="primary-action" type="button" onClick={retry}>
              Retry
            </button>
          </Show>
          <Show when={workspace.assessment().workspace.status === "unreleased"}>
            <button
              type="button"
              disabled={!allBaseAssessmentPolicyEditsPersisted(policyState()) || busy()}
              onClick={() => void validateRelease()}
            >
              {busy() ? "Checking release readiness..." : "Check release readiness"}
            </button>
            <button
              class="primary-action"
              type="button"
              disabled={!allBaseAssessmentPolicyEditsPersisted(policyState()) || busy()}
              onClick={() => void release()}
            >
              Release assessment
            </button>
          </Show>
          <Show when={policyState().persistence === "conflict"}>
            <button type="button" onClick={() => void reload()}>
              Reload server state
            </button>
          </Show>
          <A class="quiet-link" href={questionsPath}>
            Edit Questions
          </A>
          <A
            class="quiet-link"
            href={`${assessmentWorkspacePath(workspace.courseReference, workspace.assessmentReference)}/delivery-check`}
            target="_blank"
            rel="noopener"
          >
            Check assessment delivery
          </A>
        </p>
        <Show when={releaseValidation()}>
          {(validation) => (
            <section
              class="assessment-workspace-release-readiness"
              aria-labelledby="assessment-release-readiness-heading"
              role={validation().canRelease ? undefined : "alert"}
            >
              <h2 id="assessment-release-readiness-heading">
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
                            : "Enter a positive time limit in Assessment policies, save, then check release readiness again."}
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
              <Show when={!validation().canRelease}>
                <A class="quiet-link" href={questionsPath}>
                  Review assessment Questions
                </A>
              </Show>
            </section>
          )}
        </Show>
        <Show when={workspace.assessment().workspace.status === "released" && unreleaseImpact()}>
          {(impact) => (
            <section
              class="assessment-workspace-unrelease-danger-zone"
              aria-labelledby="assessment-unrelease-heading"
            >
              <h2 id="assessment-unrelease-heading">Danger Zone: Unrelease assessment</h2>
              <p>
                Unreleasing makes this Assessment unavailable to Students and permanently deletes
                its Student Work. The current Assessment definition remains available for later
                editing and release.
              </p>
              <dl aria-label="Unrelease deletion impact">
                <div>
                  <dt>Attempts</dt>
                  <dd>{impact().attemptCount}</dd>
                </div>
                <div>
                  <dt>Submissions</dt>
                  <dd>{impact().submissionCount}</dd>
                </div>
                <div>
                  <dt>Grades</dt>
                  <dd>{impact().gradeCount}</dd>
                </div>
              </dl>
              <label class="assessment-editor-field">
                Type <strong>{impact().confirmationTitle}</strong> to confirm
                <input
                  aria-describedby="assessment-unrelease-confirmation-help"
                  autocomplete="off"
                  value={confirmationTitle()}
                  onInput={(event) => setConfirmationTitle(event.currentTarget.value)}
                />
              </label>
              <p id="assessment-unrelease-confirmation-help">
                Unrelease permanently deletes the Student Work represented by these counts. No
                Student names, responses, or grade details are displayed here.
              </p>
              <button
                class="assessment-workspace-danger-action"
                type="button"
                disabled={needsReload() || confirmationTitle() !== impact().confirmationTitle}
                onClick={() => void unrelease()}
              >
                {busy() ? "Unreleasing assessment..." : "Unrelease and delete Student Work"}
              </button>
            </section>
          )}
        </Show>
      </fieldset>
    </section>
  );
}
