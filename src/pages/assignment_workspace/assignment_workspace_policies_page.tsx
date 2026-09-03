// assignment_workspace_policies_page.tsx - focused delivery-policy editor for one assignment.

import { A } from "@solidjs/router";
import { For, Show, createEffect, createMemo, createSignal, type JSX } from "solid-js";

import type { InstructorAssignmentAuthoredContentLocal } from "../../../generated/api/InstructorAssignmentAuthoredContentLocal";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentPoliciesValidationError,
} from "../../api/http_client";
import { assignmentWorkspacePath } from "./assignment_workspace_nav";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import { AssignmentWorkspacePolicyPanel } from "./assignment_workspace_policy_panel";
import {
  assignmentAvailabilityCopy,
  assignmentPolicyDraftSummary,
} from "./assignment_workspace_presentation_model";
import {
  assignmentPoliciesInput,
  assignmentPolicyCanReload,
  assignmentPolicyFeedbackDetails,
  assignmentPolicyFeedbackRole,
  assignmentPolicyFeedbackNeedsQuestionRepair,
  assignmentPoliciesValidationFeedback,
  canonicalCourseLocalTime,
  mergeSavedActivityRuleDraft,
  nonnegativeIntegerDraft,
  numberDraft,
  optionalPositiveIntegerDraft,
  activityRuleDraftFromRules,
  scoreFractionDraft,
  type AssignmentPolicyFeedback,
  type PolicyFocusTarget,
  type AssignmentActivityRuleDraft,
  type AssignmentActivityRuleDraftField,
} from "./assignment_workspace_policy_model";

function controlValue(value: string | null): string {
  return value === null ? "" : value.slice(0, 16);
}

function lateWorkRule(
  value: string,
): InstructorAssignmentAuthoredContentLocal["late_work_rule"] | undefined {
  if (value === "accept" || value === "mark_late" || value === "reject") return value;
  return undefined;
}

function relevantField(field: PolicyFocusTarget | undefined, control: PolicyFocusTarget): boolean {
  return (
    field === control ||
    (field === "schedule" && ["availableAt", "dueAt", "closesAt"].includes(control)) ||
    (field === "questions" && control === "questions")
  );
}

function fieldErrorDescription(
  field: PolicyFocusTarget | undefined,
  control: PolicyFocusTarget,
): string | undefined {
  return relevantField(field, control) ? "assignment-policies-field-error" : undefined;
}

/**
 * The page owns one local policy draft. It changes only after a successful
 * focused save so a stale revision never discards typed teaching decisions.
 */
export function AssignmentWorkspacePoliciesPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const [policies, setPolicies] = createSignal(workspace.assignment().policies);
  const [studentFeedbackReleaseRule, setStudentFeedbackReleaseRule] = createSignal(
    workspace.assignment().studentFeedbackReleaseRule,
  );
  const [assignmentAuthoredContent, setAssignmentAuthoredContent] = createSignal(
    workspace.assignment().assignmentAuthoredContent,
  );
  const [busy, setBusy] = createSignal(false);
  const [feedback, setFeedback] = createSignal<AssignmentPolicyFeedback>();
  const [failureField, setFailureField] = createSignal<PolicyFocusTarget>();
  const [needsReload, setNeedsReload] = createSignal(false);
  const [activityRuleDraft, setActivityRuleDraft] = createSignal<AssignmentActivityRuleDraft>(
    activityRuleDraftFromRules(policies()),
  );
  const [assignmentAttemptTimeLimitSecondsDraft, setAssignmentAttemptTimeLimitSecondsDraft] =
    createSignal(numberDraft(assignmentAuthoredContent().assignment_attempt_time_limit_seconds));
  const [attemptLimitDraft, setAttemptLimitDraft] = createSignal(
    numberDraft(assignmentAuthoredContent().attempt_limit),
  );
  const controls = new Map<PolicyFocusTarget, HTMLElement>();
  let saveButton!: HTMLButtonElement;
  let reloadButton: HTMLButtonElement | undefined;
  const policySummary = createMemo(() =>
    assignmentPolicyDraftSummary({
      assignmentStatus: workspace.assignment().assignmentStatus,
      savedAssignmentAvailability: workspace.assignment().assignmentAvailability,
      policies: policies(),
      activityRuleDraft: activityRuleDraft(),
      studentFeedbackReleaseRule: studentFeedbackReleaseRule(),
      assignmentAuthoredContent: assignmentAuthoredContent(),
      assignmentAttemptTimeLimitSecondsDraft: assignmentAttemptTimeLimitSecondsDraft(),
      attemptLimitDraft: attemptLimitDraft(),
    }),
  );

  const workspaceBase = (): string =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference);
  const questionsRequired = (): boolean =>
    workspace
      .assignment()
      .assignmentReleaseValidation.blockingIssues.some(
        (issue) => issue.kind === "questionsRequired",
      );
  const questionRepairRequired = (): boolean =>
    questionsRequired() || assignmentPolicyFeedbackNeedsQuestionRepair(feedback());

  function registerSaveButton(element: HTMLButtonElement): void {
    saveButton = element;
  }

  function registerReloadButton(element: HTMLButtonElement): void {
    reloadButton = element;
  }

  createEffect(() => {
    const field = failureField();
    if (field === undefined || assignmentPolicyFeedbackNeedsQuestionRepair(feedback())) return;
    const target = field === "schedule" ? "availableAt" : field;
    queueMicrotask(() => controls.get(target)?.focus());
  });

  createEffect(() => {
    if (!assignmentPolicyFeedbackNeedsQuestionRepair(feedback()) || busy()) return;
    queueMicrotask(() => controls.get("questions")?.focus());
  });

  createEffect(() => {
    const reloadIsShown = needsReload() || assignmentPolicyCanReload(feedback());
    if (!reloadIsShown || busy()) return;
    queueMicrotask(() => reloadButton?.focus());
  });

  function updateTeaching(
    next: Partial<InstructorAssignmentAuthoredContentLocal>,
    control?: PolicyFocusTarget,
  ): void {
    setAssignmentAuthoredContent((current) => ({ ...current, ...next }));
    if (control !== undefined) clearRecoveredControl(control);
  }

  function clearRecoveredField(field: PolicyFocusTarget): void {
    if (failureField() !== field) return;
    setFailureField(undefined);
    if (feedback()?.kind === "error") {
      setFeedback({
        kind: "info",
        message:
          "Your correction is ready. Save assignment policies to apply it to this assignment.",
      });
    }
  }

  /** A correction clears the matching server recovery state immediately. */
  function clearRecoveredControl(control: PolicyFocusTarget): void {
    const recovered = failureField();
    const matches =
      recovered === control ||
      (recovered === "schedule" && ["availableAt", "dueAt", "closesAt"].includes(control));
    if (matches && recovered !== undefined) clearRecoveredField(recovered);
  }

  function updateNumberDraft(
    field: "assignmentAttemptTimeLimitSeconds" | "attemptLimit",
    raw: string,
  ): void {
    const parsed = optionalPositiveIntegerDraft(raw);
    if (field === "assignmentAttemptTimeLimitSeconds")
      setAssignmentAttemptTimeLimitSecondsDraft(raw);
    else setAttemptLimitDraft(raw);
    if (!parsed.valid) return;
    updateTeaching(
      field === "assignmentAttemptTimeLimitSeconds"
        ? { assignment_attempt_time_limit_seconds: parsed.value }
        : { attempt_limit: parsed.value },
      field,
    );
  }

  function updateActivityRuleDraft(field: AssignmentActivityRuleDraftField, raw: string): void {
    setActivityRuleDraft((current) => ({ ...current, [field]: raw }));
    const parsed =
      field === "completionFraction" ? scoreFractionDraft(raw) : nonnegativeIntegerDraft(raw);
    const value = parsed.value;
    if (!parsed.valid || value === null) return;
    clearRecoveredField(field);
    if (
      field === "completionFraction" &&
      policies().assignmentCompletionRule.kind === "scoreAtLeast"
    ) {
      setPolicies((current) => ({
        ...current,
        assignmentCompletionRule: { kind: "scoreAtLeast", fraction: value },
      }));
    }
    if (
      field === "additionalAssignmentAttempts" &&
      policies().assignmentAttemptContinuationRule.kind === "capped"
    ) {
      setPolicies((current) => ({
        ...current,
        assignmentAttemptContinuationRule: {
          kind: "capped",
          maxAdditionalAssignmentAttempts: value,
        },
      }));
    }
  }

  function changeCompletionKind(
    kind: AssignmentActivityRules["assignmentCompletionRule"]["kind"],
  ): void {
    if (kind === "allCorrect") {
      setPolicies((current) => ({ ...current, assignmentCompletionRule: { kind } }));
      return;
    }
    if (kind === "answerAll") {
      setPolicies((current) => ({ ...current, assignmentCompletionRule: { kind } }));
      return;
    }
    const parsed = scoreFractionDraft(activityRuleDraft().completionFraction);
    setPolicies((current) => ({
      ...current,
      assignmentCompletionRule: { kind, fraction: parsed.value ?? 0.8 },
    }));
  }

  function changeAssignmentAttemptContinuationRuleKind(
    kind: AssignmentActivityRules["assignmentAttemptContinuationRule"]["kind"],
  ): void {
    if (kind === "unlimited" || kind === "closed") {
      setPolicies((current) => ({ ...current, assignmentAttemptContinuationRule: { kind } }));
      return;
    }
    const parsed = nonnegativeIntegerDraft(activityRuleDraft().additionalAssignmentAttempts);
    setPolicies((current) => ({
      ...current,
      assignmentAttemptContinuationRule: {
        kind,
        maxAdditionalAssignmentAttempts: parsed.value ?? 3,
      },
    }));
  }

  function activityRuleFieldError(field: AssignmentActivityRuleDraftField): string | undefined {
    const active =
      (field === "completionFraction" &&
        policies().assignmentCompletionRule.kind === "scoreAtLeast") ||
      (field === "additionalAssignmentAttempts" &&
        policies().assignmentAttemptContinuationRule.kind === "capped");
    if (!active) return undefined;
    const parsed =
      field === "completionFraction"
        ? scoreFractionDraft(activityRuleDraft().completionFraction)
        : nonnegativeIntegerDraft(activityRuleDraft().additionalAssignmentAttempts);
    if (!parsed.valid) {
      return field === "completionFraction"
        ? "Enter a decimal from 0 through 1."
        : "Enter a whole number of 0 or more.";
    }
    return relevantField(failureField(), field) ? feedback()?.message : undefined;
  }

  function deliveryNumberFieldError(
    field: "assignmentAttemptTimeLimitSeconds" | "attemptLimit",
  ): string | undefined {
    const parsed = optionalPositiveIntegerDraft(
      field === "assignmentAttemptTimeLimitSeconds"
        ? assignmentAttemptTimeLimitSecondsDraft()
        : attemptLimitDraft(),
    );
    if (!parsed.valid) {
      return `${field === "assignmentAttemptTimeLimitSeconds" ? "Whole Assignment Attempt seconds" : "Attempt limit"} must be a positive whole number or blank.`;
    }
    return relevantField(failureField(), field) ? feedback()?.message : undefined;
  }

  function questionVariationRuleError(): string | undefined {
    return relevantField(failureField(), "questionVariationRule") ? feedback()?.message : undefined;
  }

  function questionPoolReuseRuleError(): string | undefined {
    return relevantField(failureField(), "questionPoolReuseRule") ? feedback()?.message : undefined;
  }

  function updateQuestionPoolReuseRule(next: AssignmentActivityRules): void {
    setPolicies(next);
    clearRecoveredControl("questionPoolReuseRule");
  }

  function updateQuestionVariationRule(next: AssignmentActivityRules): void {
    setPolicies(next);
    clearRecoveredControl("questionVariationRule");
  }

  function firstInvalidNumericField(): PolicyFocusTarget | undefined {
    if (
      policies().assignmentCompletionRule.kind === "scoreAtLeast" &&
      !scoreFractionDraft(activityRuleDraft().completionFraction).valid
    ) {
      return "completionFraction";
    }
    if (
      policies().assignmentAttemptContinuationRule.kind === "capped" &&
      !nonnegativeIntegerDraft(activityRuleDraft().additionalAssignmentAttempts).valid
    ) {
      return "additionalAssignmentAttempts";
    }
    if (!optionalPositiveIntegerDraft(assignmentAttemptTimeLimitSecondsDraft()).valid)
      return "assignmentAttemptTimeLimitSeconds";
    if (!optionalPositiveIntegerDraft(attemptLimitDraft()).valid) return "attemptLimit";
    return undefined;
  }

  async function save(): Promise<void> {
    if (needsReload()) {
      setFeedback({
        kind: "conflict",
        message:
          "Reload the latest assignment before saving. Your typed policy edits remain here until you choose Reload latest assignment to replace them with current policies.",
      });
      return;
    }
    const invalidField = firstInvalidNumericField();
    if (invalidField !== undefined) {
      const label =
        invalidField === "completionFraction"
          ? "Required score fraction"
          : invalidField === "additionalAssignmentAttempts"
            ? "Additional Assignment Attempts"
            : invalidField === "assignmentAttemptTimeLimitSeconds"
              ? "Whole Assignment Attempt seconds"
              : "Attempt limit";
      setFailureField(invalidField);
      setFeedback({
        kind: "error",
        message: `${label} needs correction before saving.`,
      });
      return;
    }
    setBusy(true);
    setFeedback(undefined);
    setFailureField(undefined);
    try {
      const input = assignmentPoliciesInput(
        studentFeedbackReleaseRule(),
        policies(),
        assignmentAuthoredContent(),
      );
      const saved = await workspace.client.saveAssignmentPolicies(
        workspace.courseId,
        workspace.assignmentId,
        workspace.assignment().reference,
        input,
        workspace.assignment().revision,
      );
      workspace.replaceAssignment(saved);
      setPolicies(saved.policies);
      setStudentFeedbackReleaseRule(saved.studentFeedbackReleaseRule);
      setAssignmentAuthoredContent(saved.assignmentAuthoredContent);
      setAssignmentAttemptTimeLimitSecondsDraft(
        numberDraft(saved.assignmentAuthoredContent.assignment_attempt_time_limit_seconds),
      );
      setAttemptLimitDraft(numberDraft(saved.assignmentAuthoredContent.attempt_limit));
      setActivityRuleDraft((current) => mergeSavedActivityRuleDraft(current, saved.policies));
      setNeedsReload(false);
      setFeedback({
        kind: "success",
        message: "Assignment policies saved. The current assignment now uses the new revision.",
      });
    } catch (error: unknown) {
      if (error instanceof AssignmentPoliciesValidationError) {
        const nextFeedback = assignmentPoliciesValidationFeedback(error.issues);
        setFailureField(nextFeedback.target);
        setFeedback(nextFeedback);
      } else if (error instanceof AssignmentConflictError) {
        setNeedsReload(true);
        setFeedback({
          kind: "conflict",
          message:
            "This assignment changed elsewhere. Your policy edits are still here. Reload the latest assignment to replace them with the current policies before saving again.",
        });
      } else if (error instanceof ApiRequestError) {
        setFeedback({
          kind: "error",
          message: "Assignment policies were not saved. Review the settings and try again.",
        });
      } else {
        setFeedback({ kind: "error", message: "Assignment policies were not saved. Try again." });
      }
    } finally {
      setBusy(false);
    }
  }

  async function reloadLatest(): Promise<void> {
    setBusy(true);
    let reloaded = false;
    try {
      const latest = await workspace.reloadAssignment();
      setPolicies(latest.policies);
      setStudentFeedbackReleaseRule(latest.studentFeedbackReleaseRule);
      setAssignmentAuthoredContent(latest.assignmentAuthoredContent);
      setAssignmentAttemptTimeLimitSecondsDraft(
        numberDraft(latest.assignmentAuthoredContent.assignment_attempt_time_limit_seconds),
      );
      setAttemptLimitDraft(numberDraft(latest.assignmentAuthoredContent.attempt_limit));
      setActivityRuleDraft(activityRuleDraftFromRules(latest.policies));
      setFailureField(undefined);
      setNeedsReload(false);
      setFeedback({
        kind: "info",
        message:
          "Latest assignment loaded; your local policy edits were replaced. Review the current policies before saving.",
      });
      reloaded = true;
    } catch {
      setNeedsReload(true);
      setFeedback({
        kind: "conflict",
        message:
          "The latest assignment could not be loaded. Try Reload latest assignment again; your typed policy edits remain here.",
      });
    } finally {
      setBusy(false);
      if (reloaded) queueMicrotask(() => saveButton.focus());
    }
  }

  return (
    <section class="assignment-workspace-policies" aria-labelledby="assignment-policies-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-policies-heading">Policies</h1>
        <p class="page-lede">
          Configure how {workspace.assignment().title} opens, accepts Assignment Attempts, and
          shares Student Feedback. Times use the Course wall clock.
        </p>
      </header>

      <Show when={feedback()}>
        {(currentFeedback) => (
          <div
            id={failureField() === undefined ? undefined : "assignment-policies-field-error"}
            class="assignment-workspace-save-message"
            role={assignmentPolicyFeedbackRole(currentFeedback())}
            aria-live={
              assignmentPolicyFeedbackRole(currentFeedback()) === "alert" ? "assertive" : "polite"
            }
          >
            <p>{currentFeedback().message}</p>
            <Show when={assignmentPolicyFeedbackDetails(currentFeedback()).length > 0}>
              <ul>
                <For each={assignmentPolicyFeedbackDetails(currentFeedback())}>
                  {(detail) => <li>{detail}</li>}
                </For>
              </ul>
            </Show>
          </div>
        )}
      </Show>

      <section
        class="assignment-workspace-policy-summary"
        aria-labelledby="assignment-policy-summary-heading"
      >
        <h2 id="assignment-policy-summary-heading">Saved Assignment and unsaved edits</h2>
        <dl>
          <For each={policySummary()}>
            {(item) => (
              <div data-policy-summary={item.key}>
                <dt>{item.label}</dt>
                <dd>{item.value}</dd>
              </div>
            )}
          </For>
        </dl>
      </section>

      <fieldset class="assignment-workspace-policy-controls" disabled={busy()} aria-busy={busy()}>
        <legend class="visually-hidden">Assignment policy controls</legend>
        <section class="assignment-workspace-policy-actions" aria-label="Policy actions">
          <button
            ref={registerSaveButton}
            class="primary-action"
            type="button"
            disabled={needsReload() || firstInvalidNumericField() !== undefined}
            onClick={() => void save()}
          >
            {busy() ? "Saving assignment policies..." : "Save assignment policies"}
          </button>
          <Show when={needsReload() || assignmentPolicyCanReload(feedback())}>
            <button
              ref={registerReloadButton}
              type="button"
              aria-label="Reload latest assignment and replace local policy edits"
              onClick={() => void reloadLatest()}
            >
              Reload latest assignment
            </button>
          </Show>
          <A class="quiet-link" href={`${workspaceBase()}/access`}>
            Access and accommodations
          </A>
          <A class="quiet-link" href={`${workspaceBase()}/delivery-check`}>
            Check assignment delivery
          </A>
          <Show when={questionRepairRequired()}>
            <A
              ref={(element: HTMLAnchorElement) => controls.set("questions", element)}
              class="quiet-link"
              href={assignmentWorkspacePath(
                workspace.courseReference,
                workspace.assignmentReference,
                "questions",
              )}
            >
              Add at least one question
            </A>
          </Show>
        </section>

        <div class="assignment-workspace-policy-grid">
          <AssignmentWorkspacePolicyPanel
            policies={policies}
            studentFeedbackReleaseRule={studentFeedbackReleaseRule}
            activityRuleDraft={activityRuleDraft}
            activityRuleFieldError={activityRuleFieldError}
            questionPoolReuseRuleError={questionPoolReuseRuleError}
            questionVariationRuleError={questionVariationRuleError}
            onPoliciesChange={setPolicies}
            onQuestionPoolReuseRuleChange={updateQuestionPoolReuseRule}
            onQuestionVariationRuleChange={updateQuestionVariationRule}
            onStudentFeedbackReleaseRuleChange={setStudentFeedbackReleaseRule}
            onActivityRuleDraftChange={updateActivityRuleDraft}
            onCompletionKindChange={changeCompletionKind}
            onAssignmentAttemptContinuationRuleKindChange={
              changeAssignmentAttemptContinuationRuleKind
            }
            onRegisterActivityRuleControl={(field, element) => controls.set(field, element)}
            onRegisterPolicyControl={(field, element) => controls.set(field, element)}
          />

          <section
            class="assignment-editor-policy-panel assignment-editor-policy-panel--delivery"
            aria-labelledby="assignment-delivery-policies-heading"
          >
            <h2 id="assignment-delivery-policies-heading">Release and delivery</h2>
            <p class="assignment-editor-note" role="status">
              {assignmentAvailabilityCopy(
                workspace.assignment().assignmentStatus,
                workspace.assignment().assignmentAvailability,
                workspace.assignment().assignmentAuthoredContent.timeZone,
              )}
            </p>
            <p class="assignment-editor-note">
              Course time zone: {assignmentAuthoredContent().timeZone}.
            </p>

            <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--delivery">
              <legend>Student instructions</legend>
              <label class="assignment-editor-field">
                Student instructions
                <textarea
                  ref={(element) => controls.set("instructions", element)}
                  rows="4"
                  value={assignmentAuthoredContent().instructions}
                  aria-invalid={relevantField(failureField(), "instructions")}
                  aria-describedby={fieldErrorDescription(failureField(), "instructions")}
                  onInput={(event) =>
                    updateTeaching({ instructions: event.currentTarget.value }, "instructions")
                  }
                />
              </label>
            </fieldset>

            <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--schedule">
              <legend>Schedule and limits</legend>
              <label class="assignment-editor-field">
                Available
                <input
                  type="datetime-local"
                  ref={(element) => controls.set("availableAt", element)}
                  step="0.001"
                  value={controlValue(assignmentAuthoredContent().available_at)}
                  aria-invalid={relevantField(failureField(), "availableAt")}
                  aria-describedby={fieldErrorDescription(failureField(), "availableAt")}
                  onChange={(event) =>
                    updateTeaching(
                      {
                        available_at: canonicalCourseLocalTime(event.currentTarget.value),
                      },
                      "availableAt",
                    )
                  }
                />
              </label>
              <label class="assignment-editor-field">
                Due
                <input
                  type="datetime-local"
                  ref={(element) => controls.set("dueAt", element)}
                  step="0.001"
                  value={controlValue(assignmentAuthoredContent().due_at)}
                  aria-invalid={relevantField(failureField(), "dueAt")}
                  aria-describedby={fieldErrorDescription(failureField(), "dueAt")}
                  onChange={(event) =>
                    updateTeaching(
                      { due_at: canonicalCourseLocalTime(event.currentTarget.value) },
                      "dueAt",
                    )
                  }
                />
              </label>
              <label class="assignment-editor-field">
                Closes
                <input
                  type="datetime-local"
                  ref={(element) => controls.set("closesAt", element)}
                  step="0.001"
                  value={controlValue(assignmentAuthoredContent().closes_at)}
                  aria-invalid={relevantField(failureField(), "closesAt")}
                  aria-describedby={fieldErrorDescription(failureField(), "closesAt")}
                  onChange={(event) =>
                    updateTeaching(
                      { closes_at: canonicalCourseLocalTime(event.currentTarget.value) },
                      "closesAt",
                    )
                  }
                />
              </label>
              <label class="assignment-editor-field">
                Whole Assignment Attempt seconds
                <input
                  type="number"
                  ref={(element) => controls.set("assignmentAttemptTimeLimitSeconds", element)}
                  min="1"
                  aria-invalid={
                    deliveryNumberFieldError("assignmentAttemptTimeLimitSeconds") !== undefined
                  }
                  aria-describedby={
                    deliveryNumberFieldError("assignmentAttemptTimeLimitSeconds") === undefined
                      ? undefined
                      : "assignment-policies-assignmentAttemptTimeLimitSeconds-error"
                  }
                  value={assignmentAttemptTimeLimitSecondsDraft()}
                  onInput={(event) =>
                    updateNumberDraft(
                      "assignmentAttemptTimeLimitSeconds",
                      event.currentTarget.value,
                    )
                  }
                />
                <Show when={deliveryNumberFieldError("assignmentAttemptTimeLimitSeconds")}>
                  {(message) => (
                    <p
                      id="assignment-policies-assignmentAttemptTimeLimitSeconds-error"
                      class="assignment-editor-note"
                      role="status"
                    >
                      {message()}
                    </p>
                  )}
                </Show>
              </label>
              <label class="assignment-editor-field">
                Attempt limit
                <input
                  type="number"
                  ref={(element) => controls.set("attemptLimit", element)}
                  min="1"
                  aria-invalid={deliveryNumberFieldError("attemptLimit") !== undefined}
                  aria-describedby={
                    deliveryNumberFieldError("attemptLimit") === undefined
                      ? undefined
                      : "assignment-policies-attemptLimit-error"
                  }
                  value={attemptLimitDraft()}
                  onInput={(event) => updateNumberDraft("attemptLimit", event.currentTarget.value)}
                />
                <Show when={deliveryNumberFieldError("attemptLimit")}>
                  {(message) => (
                    <p
                      id="assignment-policies-attemptLimit-error"
                      class="assignment-editor-note"
                      role="status"
                    >
                      {message()}
                    </p>
                  )}
                </Show>
              </label>
              <label class="assignment-editor-field">
                Late work
                <select
                  value={assignmentAuthoredContent().late_work_rule}
                  onChange={(event) => {
                    const next = lateWorkRule(event.currentTarget.value);
                    if (next !== undefined) updateTeaching({ late_work_rule: next }, "schedule");
                  }}
                >
                  <option value="accept">Accept</option>
                  <option value="mark_late">Accept and mark late</option>
                  <option value="reject">Reject after the due time</option>
                </select>
              </label>
              <p class="assignment-editor-note">
                At the effective deadline, the server automatically submits active work.
              </p>
            </fieldset>
          </section>
        </div>
      </fieldset>
    </section>
  );
}
