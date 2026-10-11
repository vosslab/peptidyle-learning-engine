import {
  ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES,
  NO_CLOSING_TIME_GUIDANCE,
  NO_OPENING_TIME_GUIDANCE,
  OPTIONAL_DURATION_OVERRIDE_GUIDANCE,
  UNLIMITED_ATTEMPTS_GUIDANCE,
  assessmentDurationDefaultDescription,
  assessmentDurationOverrideMinutesDraft,
  assessmentDurationOverrideMinutesError,
  assessmentDurationOverrideSecondsFromMinutesDraft,
} from "../../assessment_duration";
import { Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type {
  AssessmentUnreleaseImpact,
  AssessmentReleaseValidation,
  LiveAssessmentWorkspace,
  SaveBaseAssessmentPolicyInput,
} from "../../api/assessment_release";
import { useApplicationApi } from "../../api/application_api";
import { PageFrame } from "../../components/page_frame";
import { ApiRequestError } from "../../api/http_client/error";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import {
  AssessmentPolicyFeedbackControls,
  type AssessmentFeedbackField,
} from "./assessment_policy_feedback_controls";
import { AssessmentPolicyReleaseControls } from "./assessment_policy_release_controls";
import { AssessmentFixedQuestionPointsEditor } from "./assessment_fixed_question_points_editor";
import { AssessmentStudentTimeAccommodations } from "./assessment_student_time_accommodations";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import {
  AssessmentWorkspaceIdentity,
  useAssessmentWorkspace,
} from "./assessment_workspace_live_page";
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

/** Keeps an optional local schedule bound unset until both controls are complete. */
function optionalLocalDateAndTime(date: string, time: string): string | null | undefined {
  if (date === "" && time === "") return null;
  if (date === "" || time === "") return undefined;
  return canonicalLocalDateAndTime(`${date}T${time}`) ?? undefined;
}

function optionalScheduleDateDraft(value: LiveAssessmentWorkspace["availableAt"]): string {
  return value === null ? "" : value.slice(0, 10);
}

function optionalScheduleTimeDraft(value: LiveAssessmentWorkspace["availableAt"]): string {
  return value === null ? "" : value.slice(11);
}

function optionalScheduleError(date: string, time: string, label: string): string {
  if (date === "" && time === "") return "";
  const article = /^[aeiou]/iu.test(label) ? "an" : "a";
  if (date === "") return `Choose ${article} ${label} date or clear its time.`;
  if (time === "") return `Choose ${article} ${label} time or clear its date.`;
  return canonicalLocalDateAndTime(`${date}T${time}`) === null
    ? `Enter a valid local ${label} date and time.`
    : "";
}

/** Edits the full direct resource while keeping timing and feedback controls independent. */
export function AssessmentWorkspacePoliciesPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assessment().workspace;
  const [instructions, setInstructions] = createSignal(initial.instructions);
  const [dueDate, setDueDate] = createSignal(dueDateDraft(initial.dueAt));
  const [dueTime, setDueTime] = createSignal(dueTimeDraft(initial.dueAt));
  const [availableDate, setAvailableDate] = createSignal(
    optionalScheduleDateDraft(initial.availableAt),
  );
  const [availableTime, setAvailableTime] = createSignal(
    optionalScheduleTimeDraft(initial.availableAt),
  );
  const [closesDate, setClosesDate] = createSignal(optionalScheduleDateDraft(initial.closesAt));
  const [closesTime, setClosesTime] = createSignal(optionalScheduleTimeDraft(initial.closesAt));
  const [timeLimit, setTimeLimit] = createSignal(
    assessmentDurationOverrideMinutesDraft(initial.assessmentAttemptTimeLimitSeconds),
  );
  const oneAttemptOnly = initial.assessmentType === "quiz" || initial.assessmentType === "exam";
  const [attemptLimit, setAttemptLimit] = createSignal(
    oneAttemptOnly ? "1" : (initial.attemptLimit?.toString() ?? ""),
  );
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
  const [pointEditorActive, setPointEditorActive] = createSignal(false);
  let instructionSaveTimer: number | undefined;
  let activeRequestSeq: number | undefined;

  function integer(value: string): number | null | undefined {
    if (value === "") return null;
    return /^[1-9][0-9]*$/u.test(value) ? Number(value) : undefined;
  }
  function currentInput(): SaveBaseAssessmentPolicyInput | null {
    const parsedDueAt = canonicalLocalDateAndTime(localDueDateAndTime(dueDate(), dueTime()));
    const parsedAvailableAt = optionalLocalDateAndTime(availableDate(), availableTime());
    const parsedClosesAt = optionalLocalDateAndTime(closesDate(), closesTime());
    const parsedTimeLimit = assessmentDurationOverrideSecondsFromMinutesDraft(timeLimit());
    const parsedAttemptLimit = integer(oneAttemptOnly ? "1" : attemptLimit());
    if (
      (dueDate() !== "" && parsedDueAt === null) ||
      parsedAvailableAt === undefined ||
      parsedClosesAt === undefined ||
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
      availableAt: parsedAvailableAt,
      closesAt: parsedClosesAt,
    };
  }
  function updateOrder(shuffled: boolean): void {
    setActivityRules((current) => ({
      ...current,
      assessmentQuestionOrderRule: shuffled ? "shuffled" : "authoredOrder",
    }));
  }
  function updatePartialCredit(enabled: boolean): void {
    setActivityRules((current) => ({ ...current, partialCreditEnabled: enabled }));
  }
  function updateFeedback(field: AssessmentFeedbackField, value: string): void {
    if (
      value === "during_attempt" ||
      value === "after_submit" ||
      value === "after_due" ||
      value === "after_close" ||
      value === "after_all_students_complete" ||
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
          setMessage("Assessment Properties saved. Future Attempts use the current values.");
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
            "This Assessment changed elsewhere. Reload server state before saving; your typed values remain here.",
          );
        } else if (persistence === "rejected") {
          setMessage("The server did not accept these Assessment Properties.");
        } else {
          setMessage("Assessment Properties were not saved. Retry with the current values.");
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
        assessmentDurationOverrideMinutesError(timeLimit()) ??
          "Enter complete local dates and times and positive whole-number limits, or leave them blank.",
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
  function adoptReloadedAssessment(current: LiveAssessmentWorkspace): void {
    setInstructions(current.instructions);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setAvailableDate(optionalScheduleDateDraft(current.availableAt));
    setAvailableTime(optionalScheduleTimeDraft(current.availableAt));
    setClosesDate(optionalScheduleDateDraft(current.closesAt));
    setClosesTime(optionalScheduleTimeDraft(current.closesAt));
    setTimeLimit(assessmentDurationOverrideMinutesDraft(current.assessmentAttemptTimeLimitSeconds));
    setAttemptLimit(oneAttemptOnly ? "1" : (current.attemptLimit?.toString() ?? ""));
    setLateWorkRule(current.lateWorkRule);
    setActivityRules(current.activityRules);
    setFeedbackRules(current.studentFeedbackReleaseRule);
    setPolicyState((state) =>
      baseAssessmentPolicyReloaded(state, baseAssessmentPolicyInput(current)),
    );
    setNeedsReload(false);
    setReleaseValidation(undefined);
  }
  async function reload(): Promise<boolean> {
    setBusy(true);
    try {
      const latest = await workspace.reloadAssessment();
      const current = latest.workspace;
      adoptReloadedAssessment(current);
      setMessage("Latest Assessment loaded. Review the current Properties.");
      return true;
    } catch {
      setMessage("The latest Assessment could not load. Your typed values remain here.");
      return false;
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
        workspace.courseInstanceId,
        workspace.assessmentId,
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
      const released = await workspace.release(current.workspace.assessmentEditNumber);
      await loadUnreleaseImpact();
      setReleaseValidation(undefined);
      setMessage(
        `Assessment released. Current edit number: ${released.workspace.assessmentEditNumber}.`,
      );
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssessmentWorkspaceConflictError;
      setNeedsReload(conflict);
      if (conflict) setPolicyState((state) => ({ ...state, persistence: "conflict" }));
      setMessage(
        conflict
          ? "This assessment changed elsewhere. Reload latest assessment before releasing it."
          : "The Assessment could not be released. Review its Questions and Properties, then try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function loadUnreleaseImpact(): Promise<void> {
    if (workspace.assessment().workspace.status !== "released") return;
    try {
      const impact = await applicationApi.client.getLiveAssessmentUnreleaseImpact(
        workspace.courseInstanceId,
        workspace.assessmentId,
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
    workspace.courseInstanceId,
    workspace.assessmentId,
    "questions",
  );
  return (
    <PageFrame
      // Policy editor panels. PageFrame owns the stack.
      contentClass="assessment-workspace-policies"
      routeSurface="assessmentWorkspace"
      headingId="assessment-policies-heading"
      eyebrow="Assessment workspace"
      title="Assessment Properties Editor"
    >
      <AssessmentWorkspaceIdentity />
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
      <AssessmentFixedQuestionPointsEditor
        disabled={!allBaseAssessmentPolicyEditsPersisted(policyState()) || busy() || needsReload()}
        onEditingChange={setPointEditorActive}
        reloadLatest={reload}
      />
      <fieldset
        class="assessment-workspace-policy-controls"
        disabled={busy() || pointEditorActive()}
        aria-busy={busy() || policyState().persistence === "saving"}
      >
        <legend>Assessment Properties</legend>
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
          <div
            class="assessment-workspace-schedule"
            role="group"
            aria-label="Available date and time"
          >
            <label class="assessment-editor-field">
              Available date
              <input
                type="date"
                value={availableDate()}
                aria-invalid={
                  optionalScheduleError(availableDate(), availableTime(), "available") !== ""
                }
                aria-describedby={
                  optionalScheduleError(availableDate(), availableTime(), "available") === ""
                    ? undefined
                    : "assessment-available-time-error"
                }
                onInput={(event) => {
                  setAvailableDate(event.currentTarget.value);
                  recordDraft();
                }}
              />
            </label>
            <label class="assessment-editor-field">
              Available time
              <input
                type="time"
                step="0.001"
                value={availableTime()}
                aria-invalid={
                  optionalScheduleError(availableDate(), availableTime(), "available") !== ""
                }
                aria-describedby={
                  optionalScheduleError(availableDate(), availableTime(), "available") === ""
                    ? undefined
                    : "assessment-available-time-error"
                }
                onInput={(event) => {
                  setAvailableTime(event.currentTarget.value);
                  recordDraft();
                }}
              />
            </label>
            <Show when={optionalScheduleError(availableDate(), availableTime(), "available")}>
              {(error) => (
                <small id="assessment-available-time-error" role="alert">
                  {error()}
                </small>
              )}
            </Show>
            <small>{NO_OPENING_TIME_GUIDANCE}</small>
          </div>
          <div class="assessment-workspace-schedule" role="group" aria-label="Due date and time">
            <label class="assessment-editor-field">
              Due date
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
          <div class="assessment-workspace-schedule" role="group" aria-label="Closes date and time">
            <label class="assessment-editor-field">
              Closes date
              <input
                type="date"
                value={closesDate()}
                aria-invalid={optionalScheduleError(closesDate(), closesTime(), "closing") !== ""}
                aria-describedby={
                  optionalScheduleError(closesDate(), closesTime(), "closing") === ""
                    ? undefined
                    : "assessment-closes-time-error"
                }
                onInput={(event) => {
                  setClosesDate(event.currentTarget.value);
                  recordDraft();
                }}
              />
            </label>
            <label class="assessment-editor-field">
              Closes time
              <input
                type="time"
                step="0.001"
                value={closesTime()}
                aria-invalid={optionalScheduleError(closesDate(), closesTime(), "closing") !== ""}
                aria-describedby={
                  optionalScheduleError(closesDate(), closesTime(), "closing") === ""
                    ? undefined
                    : "assessment-closes-time-error"
                }
                onInput={(event) => {
                  setClosesTime(event.currentTarget.value);
                  recordDraft();
                }}
              />
            </label>
            <Show when={optionalScheduleError(closesDate(), closesTime(), "closing")}>
              {(error) => (
                <small id="assessment-closes-time-error" role="alert">
                  {error()}
                </small>
              )}
            </Show>
            <small>{NO_CLOSING_TIME_GUIDANCE}</small>
          </div>
          <label class="assessment-editor-field">
            Assessment duration override in minutes (optional, maximum 720 minutes / 12 hours)
            <input
              type="number"
              min="1"
              max={ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES}
              step="1"
              inputmode="numeric"
              value={timeLimit()}
              aria-invalid={assessmentDurationOverrideMinutesError(timeLimit()) !== undefined}
              aria-describedby={
                assessmentDurationOverrideMinutesError(timeLimit()) === undefined
                  ? undefined
                  : "assessment-duration-override-error"
              }
              onInput={(event) => {
                setTimeLimit(event.currentTarget.value);
                recordDraft();
              }}
            />
            <small>
              {assessmentDurationDefaultDescription(
                workspace
                  .assessment()
                  .workspace.entries.reduce(
                    (count, entry) =>
                      entry.availability === "available"
                        ? count + (entry.kind === "fixedQuestion" ? 1 : entry.selectionCount)
                        : count,
                    0,
                  ),
              )}{" "}
              {OPTIONAL_DURATION_OVERRIDE_GUIDANCE} Enter a whole number of minutes from 1 to{" "}
              {ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES} for a specific override.
            </small>
            <Show when={assessmentDurationOverrideMinutesError(timeLimit())}>
              {(error) => (
                <small id="assessment-duration-override-error" role="alert">
                  {error()}
                </small>
              )}
            </Show>
          </label>
          <label class="assessment-editor-field">
            Attempt limit
            <input
              type="number"
              min="1"
              value={attemptLimit()}
              disabled={oneAttemptOnly}
              onInput={(event) => {
                setAttemptLimit(event.currentTarget.value);
                recordDraft();
              }}
            />
            <Show when={oneAttemptOnly}>
              <small>Quiz and Exam permit exactly one Assessment Attempt.</small>
            </Show>
            <Show when={!oneAttemptOnly}>
              <small>{UNLIMITED_ATTEMPTS_GUIDANCE}</small>
            </Show>
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
              checked={activityRules().partialCreditEnabled}
              onChange={(event) => {
                updatePartialCredit(event.currentTarget.checked);
                recordDraft();
              }}
            />
            <span>Award partial credit</span>
          </label>
          <p>
            When enabled, stored fractional Backend credit earns proportional points. When disabled,
            only full stored credit earns points. The Full Credit entry rule remains a separate
            setting.
          </p>
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
        <AssessmentPolicyFeedbackControls
          rules={feedbackRules()}
          onChange={(field, value) => {
            updateFeedback(field, value);
            recordDraft();
          }}
        />
        <AssessmentPolicyReleaseControls
          persistence={policyState().persistence}
          status={workspace.assessment().workspace.status}
          persisted={allBaseAssessmentPolicyEditsPersisted(policyState())}
          busy={busy()}
          needsReload={needsReload()}
          validation={releaseValidation()}
          unreleaseImpact={unreleaseImpact()}
          confirmationTitle={confirmationTitle()}
          questionsPath={questionsPath}
          studentViewPath={`${assessmentWorkspacePath(workspace.courseInstanceId, workspace.assessmentId)}/student-view`}
          onRetry={retry}
          onValidateRelease={validateRelease}
          onRelease={release}
          onReload={reload}
          onConfirmationTitleChange={setConfirmationTitle}
          onUnrelease={unrelease}
        />
      </fieldset>
      <AssessmentStudentTimeAccommodations
        ready={!busy() && !needsReload() && allBaseAssessmentPolicyEditsPersisted(policyState())}
      />
    </PageFrame>
  );
}
