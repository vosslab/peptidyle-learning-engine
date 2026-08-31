// assignment_workspace_policy_panel.tsx - workspace-owned Assignment activity-rule controls.

import { Show, type JSX } from "solid-js";

import type { StudentDisclosurePolicy } from "../../../generated/api/StudentDisclosurePolicy";
import type { StudentDisclosureTiming } from "../../../generated/api/StudentDisclosureTiming";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";

import type {
  PolicyFocusTarget,
  AssignmentActivityRuleDraft,
  AssignmentActivityRuleDraftField,
} from "./assignment_workspace_policy_model";

function gradePolicy(value: string): AssignmentActivityRules["grade"] {
  if (
    value === "first" ||
    value === "latest" ||
    value === "highest" ||
    value === "instructorSelected"
  ) {
    return value;
  }
  throw new Error("Grade policy selection is invalid");
}

function variationPolicy(value: string): AssignmentActivityRules["variation"] {
  if (value === "newSeeds" || value === "selectedProblemVariants" || value === "fullRegeneration") {
    return value;
  }
  throw new Error("Variation policy selection is invalid");
}

function disclosureTiming(value: string): StudentDisclosureTiming {
  if (
    value === "during_attempt" ||
    value === "after_submit" ||
    value === "after_due" ||
    value === "after_close" ||
    value === "never"
  ) {
    return value;
  }
  throw new Error("Disclosure timing selection is invalid");
}

const disclosureTimingOptions: ReadonlyArray<readonly [StudentDisclosureTiming, string]> = [
  ["during_attempt", "While they work"],
  ["after_submit", "After they submit"],
  ["after_due", "After the due time"],
  ["after_close", "After the close time"],
  ["never", "Never"],
];

interface AssignmentWorkspacePolicyPanelProps {
  readonly policies: () => AssignmentActivityRules;
  readonly disclosurePolicy: () => StudentDisclosurePolicy;
  readonly activityRuleDraft: () => AssignmentActivityRuleDraft;
  readonly activityRuleFieldError: (field: AssignmentActivityRuleDraftField) => string | undefined;
  readonly variationPolicyError: () => string | undefined;
  readonly onPoliciesChange: (policies: AssignmentActivityRules) => void;
  readonly onVariationChange: (policies: AssignmentActivityRules) => void;
  readonly onDisclosurePolicyChange: (policy: StudentDisclosurePolicy) => void;
  readonly onActivityRuleDraftChange: (field: AssignmentActivityRuleDraftField, raw: string) => void;
  readonly onCompletionKindChange: (kind: AssignmentActivityRules["completion"]["kind"]) => void;
  readonly onContinuedPracticeKindChange: (kind: AssignmentActivityRules["continuedPractice"]["kind"]) => void;
  readonly onRegisterActivityRuleControl: (
    field: AssignmentActivityRuleDraftField,
    element: HTMLInputElement,
  ) => void;
  readonly onRegisterPolicyControl: (field: PolicyFocusTarget, element: HTMLElement) => void;
}

/** The workspace page owns the aggregate save; this panel only owns visible Assignment activity-rule controls. */
export function AssignmentWorkspacePolicyPanel(
  props: AssignmentWorkspacePolicyPanelProps,
): JSX.Element {
  function changeDisclosure(field: keyof StudentDisclosurePolicy, value: string): void {
    props.onDisclosurePolicyChange({
      ...props.disclosurePolicy(),
      [field]: disclosureTiming(value),
    });
  }

  return (
    <section
      class="assignment-editor-policy-panel assignment-editor-policy-panel--run"
      aria-labelledby="assignment-rules-heading"
    >
      <h2 id="assignment-rules-heading">Assignment rules</h2>
      <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--completion">
        <legend>Completion requirement</legend>
        <label class="assignment-editor-field">
          Completion
          <select
            aria-label="Completion requirement"
            value={props.policies().completion.kind}
            onChange={(event) => {
              const kind = event.currentTarget.value;
              if (kind === "answerAll" || kind === "scoreAtLeast" || kind === "allCorrect") {
                props.onCompletionKindChange(kind);
              }
            }}
          >
            <option value="allCorrect">All questions correct</option>
            <option value="answerAll">Answer every question</option>
            <option value="scoreAtLeast">Reach a score threshold</option>
          </select>
        </label>
        <Show when={props.policies().completion.kind === "scoreAtLeast"}>
          <label class="assignment-editor-field">
            Required score fraction
            <input
              type="number"
              ref={(element) => props.onRegisterActivityRuleControl("completionFraction", element)}
              min="0"
              max="1"
              step="0.05"
              value={props.activityRuleDraft().completionFraction}
              aria-invalid={props.activityRuleFieldError("completionFraction") !== undefined}
              aria-describedby={
                props.activityRuleFieldError("completionFraction") === undefined
                  ? undefined
                  : "assignment-policies-completionFraction-error"
              }
              onInput={(event) =>
                props.onActivityRuleDraftChange("completionFraction", event.currentTarget.value)
              }
            />
            <FieldError
              id="assignment-policies-completionFraction-error"
              message={props.activityRuleFieldError("completionFraction")}
            />
          </label>
        </Show>
      </fieldset>
      <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--grade">
        <legend>Grade policy</legend>
        <label class="assignment-editor-field">
          Record
          <select
            aria-label="Grade policy"
            value={props.policies().grade}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                grade: gradePolicy(event.currentTarget.value),
              })
            }
          >
            <option value="highest">Highest Assignment Attempt score</option>
            <option value="latest">Latest Assignment Attempt score</option>
            <option value="first">First Assignment Attempt score</option>
            <option value="instructorSelected">Instructor-selected Assignment Attempt</option>
          </select>
        </label>
      </fieldset>
      <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--practice">
        <legend>Continued practice</legend>
        <label class="assignment-editor-field">
          After completion
          <select
            aria-label="Continued practice"
            value={props.policies().continuedPractice.kind}
            onChange={(event) => {
              const kind = event.currentTarget.value;
              if (kind === "closed" || kind === "capped" || kind === "unlimited") {
                props.onContinuedPracticeKindChange(kind);
              }
            }}
          >
            <option value="unlimited">Allow unlimited practice</option>
            <option value="capped">Limit additional Assignment Attempts</option>
            <option value="closed">Close after completion</option>
          </select>
        </label>
        <Show when={props.policies().continuedPractice.kind === "capped"}>
          <label class="assignment-editor-field">
            Additional Assignment Attempts
            <input
              type="number"
              ref={(element) => props.onRegisterActivityRuleControl("additionalRuns", element)}
              min="0"
              step="1"
              value={props.activityRuleDraft().additionalRuns}
              aria-invalid={props.activityRuleFieldError("additionalRuns") !== undefined}
              aria-describedby={
                props.activityRuleFieldError("additionalRuns") === undefined
                  ? undefined
                  : "assignment-policies-additionalRuns-error"
              }
              onInput={(event) =>
                props.onActivityRuleDraftChange("additionalRuns", event.currentTarget.value)
              }
            />
            <FieldError
              id="assignment-policies-additionalRuns-error"
              message={props.activityRuleFieldError("additionalRuns")}
            />
          </label>
        </Show>
      </fieldset>
      <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--variation">
        <legend>Variation policy</legend>
        <label class="assignment-editor-field">
          Next practice Assignment Attempt
          <select
            aria-label="Variation policy"
            ref={(element) => props.onRegisterPolicyControl("variation", element)}
            value={props.policies().variation}
            aria-invalid={props.variationPolicyError() !== undefined}
            aria-describedby={
              props.variationPolicyError() === undefined
                ? undefined
                : "assignment-policies-field-error"
            }
            onChange={(event) =>
              props.onVariationChange({
                ...props.policies(),
                variation: variationPolicy(event.currentTarget.value),
              })
            }
          >
            <option value="newSeeds">Use new seeds</option>
            <option value="selectedProblemVariants">Use selected problem variants</option>
            <option value="fullRegeneration">Fully regenerate</option>
          </select>
          <Show when={props.variationPolicyError()}>
            {(message) => (
              <p class="assignment-editor-note" role="status">
                {message()}
              </p>
            )}
          </Show>
        </label>
      </fieldset>
      <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--disclosure">
        <legend>What students can see</legend>
        <p class="assignment-editor-note">
          Due and close choices stay withheld when that time is not set. The server applies the same
          setting everywhere students see this assignment.
        </p>
        <DisclosureControl
          label="Score"
          value={props.disclosurePolicy().score}
          onChange={(value) => changeDisclosure("score", value)}
        />
        <DisclosureControl
          label="Per-item correctness"
          value={props.disclosurePolicy().per_item_correctness}
          onChange={(value) => changeDisclosure("per_item_correctness", value)}
        />
        <DisclosureControl
          label="Feedback text"
          value={props.disclosurePolicy().feedback_text}
          onChange={(value) => changeDisclosure("feedback_text", value)}
        />
        <DisclosureControl
          label="Correct answer or solution"
          value={props.disclosurePolicy().solution}
          onChange={(value) => changeDisclosure("solution", value)}
        />
        <DisclosureControl
          label="Class statistics"
          value={props.disclosurePolicy().class_statistics}
          onChange={(value) => changeDisclosure("class_statistics", value)}
        />
      </fieldset>
    </section>
  );
}

function FieldError(props: {
  readonly id: string;
  readonly message: string | undefined;
}): JSX.Element {
  return (
    <Show when={props.message}>
      {(message) => (
        <p id={props.id} class="assignment-editor-note" role="status">
          {message()}
        </p>
      )}
    </Show>
  );
}

function DisclosureControl(props: {
  readonly label: string;
  readonly value: StudentDisclosureTiming;
  readonly onChange: (value: string) => void;
}): JSX.Element {
  return (
    <label class="assignment-editor-field">
      {props.label}
      <select value={props.value} onChange={(event) => props.onChange(event.currentTarget.value)}>
        {disclosureTimingOptions.map(([value, label]) => (
          <option value={value}>{label}</option>
        ))}
      </select>
    </label>
  );
}
