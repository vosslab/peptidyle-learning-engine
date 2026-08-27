// assignment_workspace_policies_page.tsx - focused delivery-policy editor for one assignment.

import { A } from "@solidjs/router";
import { For, Show, createEffect, createSignal, onMount, type JSX } from "solid-js";

import type { CourseGroupSummaryView } from "../../../generated/api/CourseGroupSummaryView";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { RunPolicies } from "../../../generated/api/RunPolicies";
import type { AssignmentPoliciesInput } from "../../api/contracts";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentPoliciesValidationError,
} from "../../api/http_client";
import { assignmentWorkspacePath } from "./assignment_workspace_nav";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import { AssignmentWorkspacePolicyPanel } from "./assignment_workspace_policy_panel";
import { assignmentCurrentStateCopy } from "./assignment_workspace_presentation_model";
import {
  assignmentPoliciesInput,
  assignmentPolicyCanReload,
  assignmentPolicyFeedbackDetails,
  assignmentPolicyFeedbackRole,
  assignmentPolicyFeedbackNeedsQuestionRepair,
  assignmentPoliciesValidationFeedback,
  canonicalCourseLocalTime,
  hasEmptyGroupAudience,
  mergeSavedRunPolicyDraft,
  nonnegativeIntegerDraft,
  numberDraft,
  optionalPositiveIntegerDraft,
  runPolicyDraftFromPolicies,
  scoreFractionDraft,
  type AssignmentPolicyFeedback,
  type PolicyFocusTarget,
  type RunPolicyDraft,
  type RunPolicyDraftField,
} from "./assignment_workspace_policy_model";

type GroupLoadState = "loading" | "ready" | "failed";

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
  if (value === "published") return "Published - eligible for learner access";
  if (value === "closed") return "Closed - no new learner work";
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
  const [audienceDraft, setAudienceDraft] = createSignal<AssignmentPoliciesInput["audience"]>(
    workspace.assignment().audience,
  );
  const [groups, setGroups] = createSignal<ReadonlyArray<CourseGroupSummaryView>>([]);
  const [busy, setBusy] = createSignal(false);
  const [feedback, setFeedback] = createSignal<AssignmentPolicyFeedback>();
  const [failureField, setFailureField] = createSignal<PolicyFocusTarget>();
  const [needsReload, setNeedsReload] = createSignal(false);
  const [groupLoadState, setGroupLoadState] = createSignal<GroupLoadState>("loading");
  const [runPolicyDraft, setRunPolicyDraft] = createSignal<RunPolicyDraft>(
    runPolicyDraftFromPolicies(policies()),
  );
  const [timeLimitSecondsDraft, setTimeLimitSecondsDraft] = createSignal(
    numberDraft(teachingSettings().timeLimitSeconds),
  );
  const [attemptLimitDraft, setAttemptLimitDraft] = createSignal(
    numberDraft(teachingSettings().attemptLimit),
  );
  const controls = new Map<PolicyFocusTarget, HTMLElement>();

  const workspaceBase = (): string =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference);
  const questionsRequired = (): boolean =>
    workspace
      .assignment()
      .publicationReadiness.blockingIssues.some((issue) => issue.kind === "questionsRequired");
  const questionRepairRequired = (): boolean =>
    questionsRequired() || assignmentPolicyFeedbackNeedsQuestionRepair(feedback());

  onMount(() => void loadGroups());

  createEffect(() => {
    const field = failureField();
    if (field === undefined) return;
    const target =
      field === "schedule" ? "availableAt" : field === "questions" ? "lifecycle" : field;
    queueMicrotask(() => controls.get(target)?.focus());
  });

  function loadGroups(): Promise<void> {
    setGroupLoadState("loading");
    return workspace.client
      .listCourseGroups(workspace.courseId, undefined, 100)
      .then((page) => {
        setGroups(page.groups);
        setGroupLoadState("ready");
      })
      .catch(() => {
        setGroupLoadState("failed");
      });
  }

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

  function updateRunPolicyDraft(field: RunPolicyDraftField, raw: string): void {
    setRunPolicyDraft((current) => ({ ...current, [field]: raw }));
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

  function changeCompletionKind(kind: RunPolicies["completion"]["kind"]): void {
    if (kind === "allCorrect") {
      setPolicies((current) => ({ ...current, completion: { kind } }));
      return;
    }
    if (kind === "answerAll") {
      setPolicies((current) => ({ ...current, completion: { kind } }));
      return;
    }
    const parsed = scoreFractionDraft(runPolicyDraft().completionFraction);
    setPolicies((current) => ({
      ...current,
      completion: { kind, fraction: parsed.value ?? 0.8 },
    }));
  }

  function changeContinuedPracticeKind(kind: RunPolicies["continuedPractice"]["kind"]): void {
    if (kind === "unlimited" || kind === "closed") {
      setPolicies((current) => ({ ...current, continuedPractice: { kind } }));
      return;
    }
    const parsed = nonnegativeIntegerDraft(runPolicyDraft().additionalRuns);
    setPolicies((current) => ({
      ...current,
      continuedPractice: { kind, maxAdditionalRuns: parsed.value ?? 3 },
    }));
  }

  function runPolicyFieldError(field: RunPolicyDraftField): string | undefined {
    const active =
      (field === "completionFraction" && policies().completion.kind === "scoreAtLeast") ||
      (field === "additionalRuns" && policies().continuedPractice.kind === "capped");
    if (!active) return undefined;
    const parsed =
      field === "completionFraction"
        ? scoreFractionDraft(runPolicyDraft().completionFraction)
        : nonnegativeIntegerDraft(runPolicyDraft().additionalRuns);
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
      return `${field === "timeLimitSeconds" ? "Whole-run seconds" : "Attempt limit"} must be a positive whole number or blank.`;
    }
    return relevantField(failureField(), field) ? feedback()?.message : undefined;
  }

  function variationPolicyError(): string | undefined {
    return relevantField(failureField(), "variation") ? feedback()?.message : undefined;
  }

  function updateVariationPolicy(next: RunPolicies): void {
    setPolicies(next);
    clearRecoveredControl("variation");
  }

  function audienceDescription(): string | undefined {
    const descriptions = [];
    if (relevantField(failureField(), "audience")) {
      descriptions.push("assignment-policies-field-error");
    }
    if (
      audienceDraft().kind === "anyOfGroups" &&
      (hasEmptyGroupAudience(audienceDraft()) || groupLoadState() === "failed")
    ) {
      descriptions.push("assignment-audience-guidance");
    }
    return descriptions.length === 0 ? undefined : descriptions.join(" ");
  }

  function firstInvalidNumericField(): PolicyFocusTarget | undefined {
    if (
      policies().completion.kind === "scoreAtLeast" &&
      !scoreFractionDraft(runPolicyDraft().completionFraction).valid
    ) {
      return "completionFraction";
    }
    if (
      policies().continuedPractice.kind === "capped" &&
      !nonnegativeIntegerDraft(runPolicyDraft().additionalRuns).valid
    ) {
      return "additionalRuns";
    }
    if (!optionalPositiveIntegerDraft(timeLimitSecondsDraft()).valid) return "timeLimitSeconds";
    if (!optionalPositiveIntegerDraft(attemptLimitDraft()).valid) return "attemptLimit";
    return undefined;
  }

  function toggleAudienceGroup(reference: string, selected: boolean): void {
    const current = audienceDraft();
    const selectedGroups = current.kind === "anyOfGroups" ? current.groups : [];
    const groups = selected
      ? [...selectedGroups, reference]
      : selectedGroups.filter((group) => group !== reference);
    setAudienceDraft({ kind: "anyOfGroups", groups });
    if (selected && groups.length > 0) clearRecoveredField("audience");
  }

  function selectedAudienceGroup(reference: string): boolean {
    const current = audienceDraft();
    return current.kind === "anyOfGroups" && current.groups.includes(reference);
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
            ? "Additional runs"
            : invalidField === "timeLimitSeconds"
              ? "Whole-run seconds"
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
        audienceDraft(),
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
      setAudienceDraft(saved.audience);
      setTimeLimitSecondsDraft(numberDraft(saved.teachingSettings.timeLimitSeconds));
      setAttemptLimitDraft(numberDraft(saved.teachingSettings.attemptLimit));
      setRunPolicyDraft((current) => mergeSavedRunPolicyDraft(current, saved.policies));
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
    try {
      const latest = await workspace.reloadAssignment();
      setPolicies(latest.policies);
      setDisclosurePolicy(latest.disclosurePolicy);
      setTeachingSettings(latest.teachingSettings);
      setAudienceDraft(latest.audience);
      setTimeLimitSecondsDraft(numberDraft(latest.teachingSettings.timeLimitSeconds));
      setAttemptLimitDraft(numberDraft(latest.teachingSettings.attemptLimit));
      setRunPolicyDraft(runPolicyDraftFromPolicies(latest.policies));
      setFailureField(undefined);
      setNeedsReload(false);
      setFeedback({
        kind: "info",
        message:
          "Latest assignment loaded; your local policy edits were replaced. Review the current policies before saving.",
      });
    } catch {
      setNeedsReload(true);
      setFeedback({
        kind: "conflict",
        message:
          "The latest assignment could not be loaded. Try Reload latest assignment again; your typed policy edits remain here.",
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="assignment-workspace-policies" aria-labelledby="assignment-policies-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-policies-heading">Policies</h1>
        <p class="page-lede">
          Configure how {workspace.assignment().title} opens, runs, and shares feedback with
          learners. Times use the course wall clock.
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
            <Show when={assignmentPolicyFeedbackNeedsQuestionRepair(currentFeedback())}>
              <A
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
          </div>
        )}
      </Show>

      <div class="assignment-workspace-policy-grid">
        <AssignmentWorkspacePolicyPanel
          policies={policies}
          disclosurePolicy={disclosurePolicy}
          runPolicyDraft={runPolicyDraft}
          runPolicyFieldError={runPolicyFieldError}
          variationPolicyError={variationPolicyError}
          onPoliciesChange={setPolicies}
          onVariationChange={updateVariationPolicy}
          onDisclosurePolicyChange={setDisclosurePolicy}
          onRunPolicyDraftChange={updateRunPolicyDraft}
          onCompletionKindChange={changeCompletionKind}
          onContinuedPracticeKindChange={changeContinuedPracticeKind}
          onRegisterRunPolicyControl={(field, element) => controls.set(field, element)}
          onRegisterPolicyControl={(field, element) => controls.set(field, element)}
        />

        <section
          class="assignment-editor-policy-panel"
          aria-labelledby="assignment-delivery-policies-heading"
        >
          <h2 id="assignment-delivery-policies-heading">Release and delivery</h2>
          <p class="assignment-editor-note" role="status">
            {assignmentCurrentStateCopy(
              teachingSettings().lifecycle,
              workspace.assignment().currentState,
              teachingSettings().timeZone,
            )}
          </p>
          <p class="assignment-editor-note">Course time zone: {teachingSettings().timeZone}.</p>

          <fieldset class="assignment-editor-policy-set">
            <legend>Lifecycle and learner instructions</legend>
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
              Learner instructions
              <textarea
                ref={(element) => controls.set("instructions", element)}
                rows="5"
                value={teachingSettings().instructions}
                aria-invalid={relevantField(failureField(), "instructions")}
                aria-describedby={fieldErrorDescription(failureField(), "instructions")}
                onInput={(event) =>
                  updateTeaching({ instructions: event.currentTarget.value }, "instructions")
                }
              />
            </label>
          </fieldset>

          <fieldset class="assignment-editor-policy-set">
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
              Whole-run seconds
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

          <fieldset class="assignment-editor-policy-set">
            <legend>Learner audience</legend>
            <div
              role="radiogroup"
              aria-label="Learner audience"
              aria-required={audienceDraft().kind === "anyOfGroups"}
              aria-describedby={audienceDescription()}
            >
              <label class="assignment-editor-field assignment-workspace-choice">
                <input
                  type="radio"
                  name="assignment-audience"
                  checked={audienceDraft().kind === "courseWide"}
                  onChange={() => {
                    setAudienceDraft({ kind: "courseWide" });
                    clearRecoveredField("audience");
                  }}
                />
                Every enrolled learner in this course
              </label>
              <label class="assignment-editor-field assignment-workspace-choice">
                <input
                  type="radio"
                  ref={(element) => controls.set("audience", element)}
                  name="assignment-audience"
                  checked={audienceDraft().kind === "anyOfGroups"}
                  aria-invalid={
                    hasEmptyGroupAudience(audienceDraft()) ||
                    relevantField(failureField(), "audience")
                  }
                  aria-describedby={audienceDescription()}
                  onChange={() => {
                    if (audienceDraft().kind === "courseWide") {
                      setAudienceDraft({ kind: "anyOfGroups", groups: [] });
                    }
                  }}
                />
                Members of one or more course groups
              </label>
            </div>
            <Show when={audienceDraft().kind === "anyOfGroups"}>
              <div
                class="assignment-workspace-group-choices"
                aria-label="Assignment groups"
                aria-describedby={audienceDescription()}
              >
                <Show when={hasEmptyGroupAudience(audienceDraft())}>
                  <p
                    id="assignment-audience-guidance"
                    class="assignment-editor-note"
                    role="status"
                    aria-live="polite"
                  >
                    Choose one or more course groups for this audience.
                  </p>
                </Show>
                <Show when={groupLoadState() === "failed"}>
                  <p
                    id={
                      hasEmptyGroupAudience(audienceDraft())
                        ? undefined
                        : "assignment-audience-guidance"
                    }
                    class="assignment-editor-note"
                    role="alert"
                  >
                    Course groups could not load. Retry course groups to choose this audience.
                  </p>
                  <button type="button" onClick={() => void loadGroups()}>
                    Retry course groups
                  </button>
                  <A
                    class="quiet-link"
                    href={`/instructor/courses/${workspace.courseReference}/teaching-operations`}
                  >
                    Open Students and groups
                  </A>
                </Show>
                <Show when={groupLoadState() === "loading"}>
                  <p class="assignment-editor-note" role="status">
                    Loading course groups...
                  </p>
                </Show>
                <Show
                  when={groupLoadState() === "ready" && groups().length > 0}
                  fallback={
                    <Show when={groupLoadState() === "ready"}>
                      <p class="assignment-editor-note">
                        No course groups are available yet. Create a group in Students and groups,
                        or choose the course-wide audience.
                      </p>
                      <A
                        class="quiet-link"
                        href={`/instructor/courses/${workspace.courseReference}/teaching-operations`}
                      >
                        Open Students and groups
                      </A>
                    </Show>
                  }
                >
                  <For each={groups()}>
                    {(group) => (
                      <label class="assignment-workspace-choice">
                        <input
                          type="checkbox"
                          checked={selectedAudienceGroup(group.reference)}
                          onChange={(event) =>
                            toggleAudienceGroup(group.reference, event.currentTarget.checked)
                          }
                        />
                        {group.title}
                      </label>
                    )}
                  </For>
                </Show>
              </div>
            </Show>
          </fieldset>
        </section>
      </div>

      <section class="assignment-workspace-policy-actions" aria-label="Policy actions">
        <button
          class="primary-action"
          type="button"
          disabled={
            busy() ||
            needsReload() ||
            hasEmptyGroupAudience(audienceDraft()) ||
            firstInvalidNumericField() !== undefined
          }
          aria-describedby={
            hasEmptyGroupAudience(audienceDraft()) ? "assignment-audience-guidance" : undefined
          }
          onClick={() => void save()}
        >
          {busy() ? "Saving assignment policies..." : "Save assignment policies"}
        </button>
        <Show when={hasEmptyGroupAudience(audienceDraft())}>
          <p class="assignment-editor-note" role="status">
            Choose one or more course groups before saving this audience.
          </p>
        </Show>
        <Show when={needsReload() || assignmentPolicyCanReload(feedback())}>
          <button
            type="button"
            disabled={busy()}
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
    </section>
  );
}
