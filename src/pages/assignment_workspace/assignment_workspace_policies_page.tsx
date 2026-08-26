// assignment_workspace_policies_page.tsx - focused delivery-policy editor for one assignment.

import { A } from "@solidjs/router";
import { For, Show, createEffect, createSignal, onMount, type JSX } from "solid-js";

import type { CourseGroupSummaryView } from "../../../generated/api/CourseGroupSummaryView";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { AssignmentPoliciesInput } from "../../api/contracts";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentTeachingSettingsValidationError,
  AssignmentValidationError,
} from "../../api/http_client";
import { AssignmentEditorPolicyPanel } from "../assignment_editor_policy_panel";
import { assignmentCurrentStateCopy } from "../assignment_teaching_operations_panel";
import { assignmentWorkspacePath } from "./assignment_workspace_nav";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import {
  assignmentPoliciesInput,
  assignmentPolicyCanReload,
  assignmentPolicyFeedbackRole,
  canonicalCourseLocalTime,
  hasEmptyGroupAudience,
  numberDraft,
  positiveIntegerDraft,
  type AssignmentPolicyFeedback,
} from "./assignment_workspace_policy_model";

type TeachingField =
  | "lifecycle"
  | "instructions"
  | "availableAt"
  | "dueAt"
  | "closesAt"
  | "timeLimitSeconds"
  | "attemptLimit"
  | "schedule";

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

function relevantField(field: TeachingField | undefined, control: TeachingField): boolean {
  return (
    field === control ||
    (field === "schedule" && ["availableAt", "dueAt", "closesAt"].includes(control))
  );
}

function fieldErrorDescription(
  field: TeachingField | undefined,
  control: TeachingField,
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
  const [failureField, setFailureField] = createSignal<TeachingField>();
  const [needsReload, setNeedsReload] = createSignal(false);
  const [timeLimitSecondsDraft, setTimeLimitSecondsDraft] = createSignal(
    numberDraft(teachingSettings().timeLimitSeconds),
  );
  const [attemptLimitDraft, setAttemptLimitDraft] = createSignal(
    numberDraft(teachingSettings().attemptLimit),
  );
  const controls = new Map<string, HTMLElement>();

  const workspaceBase = (): string =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference);
  const questionsRequired = (): boolean =>
    workspace
      .assignment()
      .publicationReadiness.blockingIssues.some((issue) => issue.kind === "questionsRequired");

  onMount(() => {
    void workspace.client
      .listCourseGroups(workspace.courseId, undefined, 100)
      .then((page) => setGroups(page.groups))
      .catch(() =>
        setFeedback({
          kind: "error",
          message: "Course groups could not load. Assignment availability remains unchanged.",
        }),
      );
  });

  createEffect(() => {
    const field = failureField();
    if (field === undefined) return;
    const target = field === "schedule" ? "availableAt" : field;
    queueMicrotask(() => controls.get(target)?.focus());
  });

  function updateTeaching(next: Partial<InstructorAssignmentTeachingSettingsLocal>): void {
    setTeachingSettings((current) => ({ ...current, ...next }));
  }

  function updateNumberDraft(field: "timeLimitSeconds" | "attemptLimit", raw: string): void {
    const parsed = positiveIntegerDraft(raw);
    if (field === "timeLimitSeconds") setTimeLimitSecondsDraft(raw);
    else setAttemptLimitDraft(raw);
    if (!parsed.valid) {
      setFailureField(field);
      setFeedback({
        kind: "error",
        message: `${field === "timeLimitSeconds" ? "Whole-run seconds" : "Attempt limit"} must be a positive whole number or blank.`,
      });
      return;
    }
    updateTeaching(
      field === "timeLimitSeconds"
        ? { timeLimitSeconds: parsed.value }
        : { attemptLimit: parsed.value },
    );
    if (failureField() === field) {
      setFailureField(undefined);
      if (feedback()?.kind === "error") setFeedback(undefined);
    }
  }

  function toggleAudienceGroup(reference: string, selected: boolean): void {
    const current = audienceDraft();
    const selectedGroups = current.kind === "anyOfGroups" ? current.groups : [];
    const groups = selected
      ? [...selectedGroups, reference]
      : selectedGroups.filter((group) => group !== reference);
    setAudienceDraft({ kind: "anyOfGroups", groups });
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
          "Reload the latest assignment before saving. Your typed policy edits remain here until you choose Reload latest assignment.",
      });
      return;
    }
    const timeLimit = positiveIntegerDraft(timeLimitSecondsDraft());
    const attemptLimit = positiveIntegerDraft(attemptLimitDraft());
    if (!timeLimit.valid || !attemptLimit.valid) {
      const field = !timeLimit.valid ? "timeLimitSeconds" : "attemptLimit";
      setFailureField(field);
      setFeedback({
        kind: "error",
        message: `${field === "timeLimitSeconds" ? "Whole-run seconds" : "Attempt limit"} must be a positive whole number or blank before saving.`,
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
      setNeedsReload(false);
      setFeedback({
        kind: "success",
        message: "Assignment policies saved. The current assignment now uses the new revision.",
      });
    } catch (error: unknown) {
      if (error instanceof AssignmentTeachingSettingsValidationError) {
        const field = error.failure.field as TeachingField;
        setFailureField(field);
        setFeedback({ kind: "error", message: error.failure.reason });
      } else if (error instanceof AssignmentValidationError) {
        setFeedback({
          kind: "error",
          message: "The assignment settings need adjustment before they can be saved.",
        });
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
          <p
            id={failureField() === undefined ? undefined : "assignment-policies-field-error"}
            class="assignment-workspace-save-message"
            role={assignmentPolicyFeedbackRole(currentFeedback())}
            aria-live={
              assignmentPolicyFeedbackRole(currentFeedback()) === "alert" ? "assertive" : "polite"
            }
          >
            {currentFeedback().message}
          </p>
        )}
      </Show>

      <div class="assignment-workspace-policy-grid">
        <AssignmentEditorPolicyPanel
          policies={policies}
          disclosurePolicy={disclosurePolicy}
          onPoliciesChange={setPolicies}
          onDisclosurePolicyChange={setDisclosurePolicy}
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
                  if (next !== undefined) updateTeaching({ lifecycle: next });
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
                onInput={(event) => updateTeaching({ instructions: event.currentTarget.value })}
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
                  updateTeaching({
                    availableAt: canonicalCourseLocalTime(event.currentTarget.value),
                  })
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
                  updateTeaching({ dueAt: canonicalCourseLocalTime(event.currentTarget.value) })
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
                  updateTeaching({ closesAt: canonicalCourseLocalTime(event.currentTarget.value) })
                }
              />
            </label>
            <label class="assignment-editor-field">
              Whole-run seconds
              <input
                type="number"
                ref={(element) => controls.set("timeLimitSeconds", element)}
                min="1"
                aria-invalid={relevantField(failureField(), "timeLimitSeconds")}
                aria-describedby={fieldErrorDescription(failureField(), "timeLimitSeconds")}
                value={timeLimitSecondsDraft()}
                onInput={(event) =>
                  updateNumberDraft("timeLimitSeconds", event.currentTarget.value)
                }
              />
            </label>
            <label class="assignment-editor-field">
              Attempt limit
              <input
                type="number"
                ref={(element) => controls.set("attemptLimit", element)}
                min="1"
                aria-invalid={relevantField(failureField(), "attemptLimit")}
                aria-describedby={fieldErrorDescription(failureField(), "attemptLimit")}
                value={attemptLimitDraft()}
                onInput={(event) => updateNumberDraft("attemptLimit", event.currentTarget.value)}
              />
            </label>
            <label class="assignment-editor-field">
              Late work
              <select
                value={teachingSettings().lateSubmission}
                onChange={(event) => {
                  const next = lateSubmission(event.currentTarget.value);
                  if (next !== undefined) updateTeaching({ lateSubmission: next });
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
            <label class="assignment-editor-field assignment-workspace-choice">
              <input
                type="radio"
                name="assignment-audience"
                checked={audienceDraft().kind === "courseWide"}
                onChange={() => setAudienceDraft({ kind: "courseWide" })}
              />
              Every enrolled learner in this course
            </label>
            <label class="assignment-editor-field assignment-workspace-choice">
              <input
                type="radio"
                name="assignment-audience"
                checked={audienceDraft().kind === "anyOfGroups"}
                onChange={() => {
                  if (audienceDraft().kind === "courseWide")
                    setAudienceDraft({ kind: "anyOfGroups", groups: [] });
                }}
              />
              Members of one or more course groups
            </label>
            <Show when={audienceDraft().kind === "anyOfGroups"}>
              <div class="assignment-workspace-group-choices" aria-label="Assignment groups">
                <Show
                  when={groups().length > 0}
                  fallback={
                    <p class="assignment-editor-note">No course groups are available yet.</p>
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
          disabled={busy() || needsReload() || hasEmptyGroupAudience(audienceDraft())}
          onClick={() => void save()}
        >
          {busy() ? "Saving assignment policies..." : "Save assignment policies"}
        </button>
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
        <Show when={questionsRequired()}>
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
