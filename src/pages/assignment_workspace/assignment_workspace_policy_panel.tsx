// assignment_workspace_policy_panel.tsx - workspace-owned run-policy controls.

import { Show, type JSX } from "solid-js";

import type { LearnerDisclosurePolicy } from "../../../generated/api/LearnerDisclosurePolicy";
import type { LearnerDisclosureTiming } from "../../../generated/api/LearnerDisclosureTiming";
import type { RunPolicies } from "../../../generated/api/RunPolicies";

import type {
  PolicyFocusTarget,
  RunPolicyDraft,
  RunPolicyDraftField,
} from "./assignment_workspace_policy_model";

function gradePolicy(value: string): RunPolicies["grade"] {
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

function variationPolicy(value: string): RunPolicies["variation"] {
  if (value === "newSeeds" || value === "selectedProblemVariants" || value === "fullRegeneration") {
    return value;
  }
  throw new Error("Variation policy selection is invalid");
}

function disclosureTiming(value: string): LearnerDisclosureTiming {
  if (
    value === "duringAttempt" ||
    value === "afterSubmit" ||
    value === "afterDue" ||
    value === "afterClose" ||
    value === "never"
  ) {
    return value;
  }
  throw new Error("Disclosure timing selection is invalid");
}

const disclosureTimingOptions: ReadonlyArray<readonly [LearnerDisclosureTiming, string]> = [
  ["duringAttempt", "While they work"],
  ["afterSubmit", "After they submit"],
  ["afterDue", "After the due time"],
  ["afterClose", "After the close time"],
  ["never", "Never"],
];

interface AssignmentWorkspacePolicyPanelProps {
  readonly policies: () => RunPolicies;
  readonly disclosurePolicy: () => LearnerDisclosurePolicy;
  readonly runPolicyDraft: () => RunPolicyDraft;
  readonly runPolicyFieldError: (field: RunPolicyDraftField) => string | undefined;
  readonly variationPolicyError: () => string | undefined;
  readonly onPoliciesChange: (policies: RunPolicies) => void;
  readonly onVariationChange: (policies: RunPolicies) => void;
  readonly onDisclosurePolicyChange: (policy: LearnerDisclosurePolicy) => void;
  readonly onRunPolicyDraftChange: (field: RunPolicyDraftField, raw: string) => void;
  readonly onCompletionKindChange: (kind: RunPolicies["completion"]["kind"]) => void;
  readonly onContinuedPracticeKindChange: (kind: RunPolicies["continuedPractice"]["kind"]) => void;
  readonly onRegisterRunPolicyControl: (
    field: RunPolicyDraftField,
    element: HTMLInputElement,
  ) => void;
  readonly onRegisterPolicyControl: (field: PolicyFocusTarget, element: HTMLElement) => void;
}

/** The workspace page owns the aggregate save; this panel only owns visible run-policy controls. */
export function AssignmentWorkspacePolicyPanel(
  props: AssignmentWorkspacePolicyPanelProps,
): JSX.Element {
  function changeDisclosure(field: keyof LearnerDisclosurePolicy, value: string): void {
    props.onDisclosurePolicyChange({
      ...props.disclosurePolicy(),
      [field]: disclosureTiming(value),
    });
  }

  return (
    <section
      class="assignment-editor-policy-panel"
      aria-labelledby="assignment-run-policies-heading"
    >
      <h2 id="assignment-run-policies-heading">Run policies</h2>
      <fieldset class="assignment-editor-policy-set">
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
              ref={(element) => props.onRegisterRunPolicyControl("completionFraction", element)}
              min="0"
              max="1"
              step="0.05"
              value={props.runPolicyDraft().completionFraction}
              aria-invalid={props.runPolicyFieldError("completionFraction") !== undefined}
              aria-describedby={
                props.runPolicyFieldError("completionFraction") === undefined
                  ? undefined
                  : "assignment-policies-completionFraction-error"
              }
              onInput={(event) =>
                props.onRunPolicyDraftChange("completionFraction", event.currentTarget.value)
              }
            />
            <FieldError
              id="assignment-policies-completionFraction-error"
              message={props.runPolicyFieldError("completionFraction")}
            />
          </label>
        </Show>
      </fieldset>
      <fieldset class="assignment-editor-policy-set">
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
            <option value="highest">Highest run score</option>
            <option value="latest">Latest run score</option>
            <option value="first">First run score</option>
            <option value="instructorSelected">Instructor-selected run</option>
          </select>
        </label>
      </fieldset>
      <fieldset class="assignment-editor-policy-set">
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
            <option value="capped">Limit additional runs</option>
            <option value="closed">Close after completion</option>
          </select>
        </label>
        <Show when={props.policies().continuedPractice.kind === "capped"}>
          <label class="assignment-editor-field">
            Additional runs
            <input
              type="number"
              ref={(element) => props.onRegisterRunPolicyControl("additionalRuns", element)}
              min="0"
              step="1"
              value={props.runPolicyDraft().additionalRuns}
              aria-invalid={props.runPolicyFieldError("additionalRuns") !== undefined}
              aria-describedby={
                props.runPolicyFieldError("additionalRuns") === undefined
                  ? undefined
                  : "assignment-policies-additionalRuns-error"
              }
              onInput={(event) =>
                props.onRunPolicyDraftChange("additionalRuns", event.currentTarget.value)
              }
            />
            <FieldError
              id="assignment-policies-additionalRuns-error"
              message={props.runPolicyFieldError("additionalRuns")}
            />
          </label>
        </Show>
      </fieldset>
      <fieldset class="assignment-editor-policy-set">
        <legend>Variation policy</legend>
        <label class="assignment-editor-field">
          Next practice run
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
      <fieldset class="assignment-editor-policy-set">
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
          value={props.disclosurePolicy().perItemCorrectness}
          onChange={(value) => changeDisclosure("perItemCorrectness", value)}
        />
        <DisclosureControl
          label="Feedback text"
          value={props.disclosurePolicy().feedbackText}
          onChange={(value) => changeDisclosure("feedbackText", value)}
        />
        <DisclosureControl
          label="Correct answer or solution"
          value={props.disclosurePolicy().solution}
          onChange={(value) => changeDisclosure("solution", value)}
        />
        <DisclosureControl
          label="Class statistics"
          value={props.disclosurePolicy().classStatistics}
          onChange={(value) => changeDisclosure("classStatistics", value)}
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
  readonly value: LearnerDisclosureTiming;
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
