// assignment_editor_policy_panel.tsx - course-owned run policy controls.

import { Show, type JSX } from "solid-js";

import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { LearnerDisclosurePolicy } from "../../generated/api/LearnerDisclosurePolicy";
import type { LearnerDisclosureTiming } from "../../generated/api/LearnerDisclosureTiming";

function completionFraction(policies: RunPolicies): number {
  return policies.completion.kind === "scoreAtLeast" ? policies.completion.fraction : 0.8;
}

function additionalRunLimit(policies: RunPolicies): number {
  return policies.continuedPractice.kind === "capped"
    ? policies.continuedPractice.maxAdditionalRuns
    : 3;
}

function gradePolicy(value: string): RunPolicies["grade"] {
  switch (value) {
    case "first":
    case "latest":
    case "highest":
    case "instructorSelected":
      return value;
    default:
      throw new Error("Grade policy selection is invalid");
  }
}

function variationPolicy(value: string): RunPolicies["variation"] {
  switch (value) {
    case "newSeeds":
    case "selectedProblemVariants":
    case "fullRegeneration":
      return value;
    default:
      throw new Error("Variation policy selection is invalid");
  }
}

function disclosureTiming(value: string): LearnerDisclosureTiming {
  switch (value) {
    case "duringAttempt":
    case "afterSubmit":
    case "afterDue":
    case "afterClose":
    case "never":
      return value;
    default:
      throw new Error("Disclosure timing selection is invalid");
  }
}

const disclosureTimingOptions: ReadonlyArray<readonly [LearnerDisclosureTiming, string]> = [
  ["duringAttempt", "While they work"],
  ["afterSubmit", "After they submit"],
  ["afterDue", "After the due time"],
  ["afterClose", "After the close time"],
  ["never", "Never"],
];

interface AssignmentEditorPolicyPanelProps {
  readonly policies: () => RunPolicies;
  readonly disclosurePolicy: () => LearnerDisclosurePolicy;
  readonly runTimed: () => boolean;
  readonly runMinutesText: () => string;
  readonly runTimingError: () => string | null;
  readonly onPoliciesChange: (policies: RunPolicies) => void;
  readonly onDisclosurePolicyChange: (policy: LearnerDisclosurePolicy) => void;
  readonly onRunTimedChange: (timed: boolean) => void;
  readonly onRunMinutesInput: (value: string) => void;
  readonly onRunMinutesInputRef: (element: HTMLInputElement) => void;
}

/**
 * Keeps the policy fieldsets as one reactive composition while the page owns
 * the revisioned assignment draft and save transaction.
 */
export function AssignmentEditorPolicyPanel(props: AssignmentEditorPolicyPanelProps): JSX.Element {
  let runMinutesInput: HTMLInputElement | undefined;

  function selectTimed(): void {
    props.onRunTimedChange(true);
    requestAnimationFrame(() => {
      runMinutesInput?.focus();
    });
  }

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
              const completion =
                kind === "answerAll"
                  ? { kind: "answerAll" as const }
                  : kind === "scoreAtLeast"
                    ? { kind: "scoreAtLeast" as const, fraction: 0.8 }
                    : { kind: "allCorrect" as const };
              props.onPoliciesChange({ ...props.policies(), completion });
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
              min="0"
              max="1"
              step="0.05"
              value={completionFraction(props.policies())}
              onInput={(event) => {
                const fraction = Number(event.currentTarget.value);
                if (!Number.isFinite(fraction)) return;
                props.onPoliciesChange({
                  ...props.policies(),
                  completion: { kind: "scoreAtLeast", fraction },
                });
              }}
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
              const continuedPractice =
                kind === "closed"
                  ? { kind: "closed" as const }
                  : kind === "capped"
                    ? { kind: "capped" as const, maxAdditionalRuns: 3 }
                    : { kind: "unlimited" as const };
              props.onPoliciesChange({ ...props.policies(), continuedPractice });
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
              min="0"
              step="1"
              value={additionalRunLimit(props.policies())}
              onInput={(event) => {
                const maxAdditionalRuns = Number(event.currentTarget.value);
                if (!Number.isSafeInteger(maxAdditionalRuns) || maxAdditionalRuns < 0) return;
                props.onPoliciesChange({
                  ...props.policies(),
                  continuedPractice: { kind: "capped", maxAdditionalRuns },
                });
              }}
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
            value={props.policies().variation}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                variation: variationPolicy(event.currentTarget.value),
              })
            }
          >
            <option value="newSeeds">Use new seeds</option>
            <option value="selectedProblemVariants">Use selected problem variants</option>
            <option value="fullRegeneration">Fully regenerate</option>
          </select>
        </label>
      </fieldset>
      <fieldset class="assignment-editor-policy-set">
        <legend>What students can see</legend>
        <p class="assignment-editor-note">
          Due and close choices stay withheld when that time is not set. The server applies the same
          setting everywhere students see this assignment.
        </p>
        <label class="assignment-editor-field">
          Score
          <select
            value={props.disclosurePolicy().score}
            onChange={(event) => changeDisclosure("score", event.currentTarget.value)}
          >
            <ForDisclosureTimingOptions />
          </select>
        </label>
        <label class="assignment-editor-field">
          Per-item correctness
          <select
            value={props.disclosurePolicy().perItemCorrectness}
            onChange={(event) => changeDisclosure("perItemCorrectness", event.currentTarget.value)}
          >
            <ForDisclosureTimingOptions />
          </select>
        </label>
        <label class="assignment-editor-field">
          Feedback text
          <select
            value={props.disclosurePolicy().feedbackText}
            onChange={(event) => changeDisclosure("feedbackText", event.currentTarget.value)}
          >
            <ForDisclosureTimingOptions />
          </select>
        </label>
        <label class="assignment-editor-field">
          Correct answer or solution
          <select
            value={props.disclosurePolicy().solution}
            onChange={(event) => changeDisclosure("solution", event.currentTarget.value)}
          >
            <ForDisclosureTimingOptions />
          </select>
        </label>
        <label class="assignment-editor-field">
          Class statistics
          <select
            value={props.disclosurePolicy().classStatistics}
            onChange={(event) => changeDisclosure("classStatistics", event.currentTarget.value)}
          >
            <ForDisclosureTimingOptions />
          </select>
        </label>
      </fieldset>
      <fieldset class="assignment-editor-policy-set assignment-editor-run-timing">
        <legend>Time limit for each practice run</legend>
        <label class="assignment-editor-radio">
          <input
            type="radio"
            name="assignment-run-timing"
            checked={props.runTimed()}
            onChange={selectTimed}
          />
          Timed
        </label>
        <Show when={props.runTimed()}>
          <label class="assignment-editor-field">
            Minutes per practice run
            <input
              ref={(element: HTMLInputElement) => {
                runMinutesInput = element;
                props.onRunMinutesInputRef(element);
              }}
              type="number"
              inputMode="decimal"
              min="0"
              step="any"
              value={props.runMinutesText()}
              aria-describedby="run-time-limit-help"
              aria-invalid={props.runTimingError() !== null}
              onInput={(event) => props.onRunMinutesInput(event.currentTarget.value)}
            />
          </label>
          <Show when={props.runTimingError()}>
            {(message) => (
              <p class="inline-error" role="alert">
                {message()}
              </p>
            )}
          </Show>
        </Show>
        <label class="assignment-editor-radio">
          <input
            type="radio"
            name="assignment-run-timing"
            checked={!props.runTimed()}
            onChange={() => props.onRunTimedChange(false)}
          />
          Untimed
        </label>
        <p id="run-time-limit-help" class="assignment-editor-note">
          The server keeps the time running and automatically submits work when the run ends. A
          loaded time may display as an approximate number of minutes; it stays exact until you edit
          this field.
        </p>
      </fieldset>
      <p class="assignment-editor-note">
        This setting limits the whole practice run. Each published question keeps its own response
        and feedback rules.
      </p>
    </section>
  );
}

function ForDisclosureTimingOptions(): JSX.Element {
  return (
    <>
      {disclosureTimingOptions.map(([value, label]) => (
        <option value={value}>{label}</option>
      ))}
    </>
  );
}
