// assignment_workspace_policies_page.tsx - focused delivery-policy editor for one assignment.

import { A } from "@solidjs/router";
import { For, Show, createEffect, createMemo, createSignal, type JSX } from "solid-js";

import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
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
  assignmentCurrentStateCopy,
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

function lifecycle(
  value: string,
): InstructorAssignmentTeachingSettingsLocal["lifecycle"] | undefined {
  if (value === "draft" || value === "published" || value === "closed" || value === "archived")
    return value;
  return undefined;
}

function lateSubmission(
  value: string,
): InstructorAssignmentTeachingSettingsLocal["lateSubmission"] | undefined {
  if (value === "accept" || value === "markLate" || value === "reject") return value;
  return undefined;
}

function lifecycleLabel(value: InstructorAssignmentTeachingSettingsLocal["lifecycle"]): string {
  if (value === "draft") return "Draft - students cannot access it";
  if (value === "published") return "Published - eligible for Student access";
  if (value === "closed") return "Closed - no new Student work";
  return "Archived - permanently retired";
}

function lifecycleChoices(
  value: InstructorAssignmentTeachingSettingsLocal["lifecycle"],
): ReadonlyArray<InstructorAssignmentTeachingSettingsLocal["lifecycle"]> {
  if (value === "draft") return ["draft", "published", "archived"];
  if (value === "published") return ["published", "closed", "archived"];
  if (value === "closed") return ["closed", "published", "archived"];
  return ["archived"];
}

function relevantField(field: PolicyFocusTarget | undefined, control: PolicyFocusTarget): boolean {
  return (
    field === control ||
    (field === "schedule" && ["availableAt", "dueAt", "closesAt"].includes(control)) ||
    (field === "questions" && control === "lifecycle")
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
  const [disclosurePolicy, setDisclosurePolicy] = createSignal(
    workspace.assignment().disclosurePolicy,
  );
  const [teachingSettings, setTeachingSettings] = createSignal(
    workspace.assignment().teachingSettings,
  );
  const [busy, setBusy] = createSignal(false);
  const [feedback, setFeedback] = createSignal<AssignmentPolicyFeedback>();
  const [failureField, setFailureField] = createSignal<PolicyFocusTarget>();
  const [needsReload, setNeedsReload] = createSignal(false);
  const [activityRuleDraft, setActivityRuleDraft] = createSignal<AssignmentActivityRuleDraft>(
    activityRuleDraftFromRules(policies()),
  );
  const [timeLimitSecondsDraft, setTimeLimitSecondsDraft] = createSignal(
    numberDraft(teachingSettings().timeLimitSeconds),
  );
  const [attemptLimitDraft, setAttemptLimitDraft] = createSignal(
    numberDraft(teachingSettings().attemptLimit),
  );
  const controls = new Map<PolicyFocusTarget, HTMLElement>();
  let saveButton!: HTMLButtonElement;
  let reloadButton: HTMLButtonElement | undefined;
  const policySummary = createMemo(() =>
    assignmentPolicyDraftSummary({
      savedLifecycle: workspace.assignment().teachingSettings.lifecycle,
      savedCurrentState: workspace.assignment().currentState,
      policies: policies(),
      activityRuleDraft: activityRuleDraft(),
      disclosurePolicy: disclosurePolicy(),
      teachingSettings: teachingSettings(),
      timeLimitSecondsDraft: timeLimitSecondsDraft(),
      attemptLimitDraft: attemptLimitDraft(),
    }),
  );

  const workspaceBase = (): string =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference);
  const questionsRequired = (): boolean =>
    workspace
      .assignment()
      .publicationReadiness.blockingIssues.some((issue) => issue.kind === "questionsRequired");
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
    next: Partial<InstructorAssignmentTeachingSettingsLocal>,
    control?: PolicyFocusTarget,
  ): void {
    setTeachingSettings((current) => ({ ...current, ...next }));
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

  function updateNumberDraft(field: "timeLimitSeconds" | "attemptLimit", raw: string): void {
    const parsed = optionalPositiveIntegerDraft(raw);
    if (field === "timeLimitSeconds") setTimeLimitSecondsDraft(raw);
    else setAttemptLimitDraft(raw);
    if (!parsed.valid) return;
    updateTeaching(
      field === "timeLimitSeconds"
        ? { timeLimitSeconds: parsed.value }
        : { attemptLimit: parsed.value },
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
    if (field === "completionFraction" && policies().completion.kind === "scoreAtLeast") {
      setPolicies((current) => ({
        ...current,
        completion: { kind: "scoreAtLeast", fraction: value },
      }));
    }
    if (field === "additionalRuns" && policies().continuedPractice.kind === "capped") {
      setPolicies((current) => ({
        ...current,
        continuedPractice: { kind: "capped", maxAdditionalRuns: value },
      }));
    }
  }

  function changeCompletionKind(kind: AssignmentActivityRules["completion"]["kind"]): void {
    if (kind === "allCorrect") {
      setPolicies((current) => ({ ...current, completion: { kind } }));
      return;
    }
    if (kind === "answerAll") {
      setPolicies((current) => ({ ...current, completion: { kind } }));
      return;
    }
    const parsed = scoreFractionDraft(activityRuleDraft().completionFraction);
    setPolicies((current) => ({
      ...current,
      completion: { kind, fraction: parsed.value ?? 0.8 },
    }));
  }

  function changeContinuedPracticeKind(kind: AssignmentActivityRules["continuedPractice"]["kind"]): void {
    if (kind === "unlimited" || kind === "closed") {
      setPolicies((current) => ({ ...current, continuedPractice: { kind } }));
      return;
    }
    const parsed = nonnegativeIntegerDraft(activityRuleDraft().additionalRuns);
    setPolicies((current) => ({
      ...current,
      continuedPractice: { kind, maxAdditionalRuns: parsed.value ?? 3 },
    }));
  }

  function activityRuleFieldError(field: AssignmentActivityRuleDraftField): string | undefined {
    const active =
      (field === "completionFraction" && policies().completion.kind === "scoreAtLeast") ||
      (field === "additionalRuns" && policies().continuedPractice.kind === "capped");
    if (!active) return undefined;
    const parsed =
      field === "completionFraction"
        ? scoreFractionDraft(activityRuleDraft().completionFraction)
        : nonnegativeIntegerDraft(activityRuleDraft().additionalRuns);
    if (!parsed.valid) {
      return field === "completionFraction"
        ? "Enter a decimal from 0 through 1."
        : "Enter a whole number of 0 or more.";
    }
    return relevantField(failureField(), field) ? feedback()?.message : undefined;
  }

  function deliveryNumberFieldError(
    field: "timeLimitSeconds" | "attemptLimit",
  ): string | undefined {
    const parsed = optionalPositiveIntegerDraft(
      field === "timeLimitSeconds" ? timeLimitSecondsDraft() : attemptLimitDraft(),
    );
    if (!parsed.valid) {
      return `${field === "timeLimitSeconds" ? "Whole Assignment Attempt seconds" : "Attempt limit"} must be a positive whole number or blank.`;
    }
    return relevantField(failureField(), field) ? feedback()?.message : undefined;
  }

  function variationPolicyError(): string | undefined {
    return relevantField(failureField(), "variation") ? feedback()?.message : undefined;
  }

  function updateVariationPolicy(next: AssignmentActivityRules): void {
    setPolicies(next);
    clearRecoveredControl("variation");
  }

  function firstInvalidNumericField(): PolicyFocusTarget | undefined {
    if (
      policies().completion.kind === "scoreAtLeast" &&
      !scoreFractionDraft(activityRuleDraft().completionFraction).valid
    ) {
      return "completionFraction";
    }
    if (
      policies().continuedPractice.kind === "capped" &&
      !nonnegativeIntegerDraft(activityRuleDraft().additionalRuns).valid
    ) {
      return "additionalRuns";
    }
    if (!optionalPositiveIntegerDraft(timeLimitSecondsDraft()).valid) return "timeLimitSeconds";
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
          : invalidField === "additionalRuns"
            ? "Additional Assignment Attempts"
            : invalidField === "timeLimitSeconds"
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
        disclosurePolicy(),
        policies(),
        teachingSettings(),
      );
      const saved = await workspace.client.saveAssignmentPolicies(
        workspace.courseId,
        workspace.assignmentId,
        input,
        workspace.assignment().revision,
      );
      workspace.replaceAssignment(saved);
      setPolicies(saved.policies);
      setDisclosurePolicy(saved.disclosurePolicy);
      setTeachingSettings(saved.teachingSettings);
      setTimeLimitSecondsDraft(numberDraft(saved.teachingSettings.timeLimitSeconds));
      setAttemptLimitDraft(numberDraft(saved.teachingSettings.attemptLimit));
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
      setDisclosurePolicy(latest.disclosurePolicy);
      setTeachingSettings(latest.teachingSettings);
      setTimeLimitSecondsDraft(numberDraft(latest.teachingSettings.timeLimitSeconds));
      setAttemptLimitDraft(numberDraft(latest.teachingSettings.attemptLimit));
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
          Configure how {workspace.assignment().title} opens, accepts Assignment Attempts, and shares
          Student Feedback. Times use the Course wall clock.
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
        <h2 id="assignment-policy-summary-heading">Saved state and current draft</h2>
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
            disabled={
              needsReload() ||
              firstInvalidNumericField() !== undefined
            }
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
            disclosurePolicy={disclosurePolicy}
            activityRuleDraft={activityRuleDraft}
            activityRuleFieldError={activityRuleFieldError}
            variationPolicyError={variationPolicyError}
            onPoliciesChange={setPolicies}
            onVariationChange={updateVariationPolicy}
            onDisclosurePolicyChange={setDisclosurePolicy}
            onActivityRuleDraftChange={updateActivityRuleDraft}
            onCompletionKindChange={changeCompletionKind}
            onContinuedPracticeKindChange={changeContinuedPracticeKind}
            onRegisterActivityRuleControl={(field, element) => controls.set(field, element)}
            onRegisterPolicyControl={(field, element) => controls.set(field, element)}
          />

          <section
            class="assignment-editor-policy-panel assignment-editor-policy-panel--delivery"
            aria-labelledby="assignment-delivery-policies-heading"
          >
            <h2 id="assignment-delivery-policies-heading">Release and delivery</h2>
            <p class="assignment-editor-note" role="status">
              {assignmentCurrentStateCopy(
                workspace.assignment().teachingSettings.lifecycle,
                workspace.assignment().currentState,
                workspace.assignment().teachingSettings.timeZone,
              )}
            </p>
            <p class="assignment-editor-note">Course time zone: {teachingSettings().timeZone}.</p>

            <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--lifecycle">
              <legend>Lifecycle and Student instructions</legend>
              <label class="assignment-editor-field">
                Lifecycle
                <select
                  ref={(element) => controls.set("lifecycle", element)}
                  value={teachingSettings().lifecycle}
                  aria-invalid={relevantField(failureField(), "lifecycle")}
                  aria-describedby={fieldErrorDescription(failureField(), "lifecycle")}
                  disabled={workspace.assignment().teachingSettings.lifecycle === "archived"}
                  onChange={(event) => {
                    const next = lifecycle(event.currentTarget.value);
                    if (next !== undefined) updateTeaching({ lifecycle: next }, "lifecycle");
                  }}
                >
                  <For each={lifecycleChoices(workspace.assignment().teachingSettings.lifecycle)}>
                    {(choice) => <option value={choice}>{lifecycleLabel(choice)}</option>}
                  </For>
                </select>
              </label>
              <label class="assignment-editor-field">
                Student instructions
                <textarea
                  ref={(element) => controls.set("instructions", element)}
                  rows="4"
                  value={teachingSettings().instructions}
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
                  value={controlValue(teachingSettings().availableAt)}
                  aria-invalid={relevantField(failureField(), "availableAt")}
                  aria-describedby={fieldErrorDescription(failureField(), "availableAt")}
                  onChange={(event) =>
                    updateTeaching(
                      {
                        availableAt: canonicalCourseLocalTime(event.currentTarget.value),
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
                  value={controlValue(teachingSettings().dueAt)}
                  aria-invalid={relevantField(failureField(), "dueAt")}
                  aria-describedby={fieldErrorDescription(failureField(), "dueAt")}
                  onChange={(event) =>
                    updateTeaching(
                      { dueAt: canonicalCourseLocalTime(event.currentTarget.value) },
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
                  value={controlValue(teachingSettings().closesAt)}
                  aria-invalid={relevantField(failureField(), "closesAt")}
                  aria-describedby={fieldErrorDescription(failureField(), "closesAt")}
                  onChange={(event) =>
                    updateTeaching(
                      { closesAt: canonicalCourseLocalTime(event.currentTarget.value) },
                      "closesAt",
                    )
                  }
                />
              </label>
              <label class="assignment-editor-field">
                Whole Assignment Attempt seconds
                <input
                  type="number"
                  ref={(element) => controls.set("timeLimitSeconds", element)}
                  min="1"
                  aria-invalid={deliveryNumberFieldError("timeLimitSeconds") !== undefined}
                  aria-describedby={
                    deliveryNumberFieldError("timeLimitSeconds") === undefined
                      ? undefined
                      : "assignment-policies-timeLimitSeconds-error"
                  }
                  value={timeLimitSecondsDraft()}
                  onInput={(event) =>
                    updateNumberDraft("timeLimitSeconds", event.currentTarget.value)
                  }
                />
                <Show when={deliveryNumberFieldError("timeLimitSeconds")}>
                  {(message) => (
                    <p
                      id="assignment-policies-timeLimitSeconds-error"
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
                  value={teachingSettings().lateSubmission}
                  onChange={(event) => {
                    const next = lateSubmission(event.currentTarget.value);
                    if (next !== undefined) updateTeaching({ lateSubmission: next }, "schedule");
                  }}
                >
                  <option value="accept">Accept</option>
                  <option value="markLate">Accept and mark late</option>
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
