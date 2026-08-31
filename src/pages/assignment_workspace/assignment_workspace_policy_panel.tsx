// assignment_workspace_policy_panel.tsx - workspace-owned Assignment activity-rule controls.

import { Show, type JSX } from "solid-js";

import type { StudentFeedbackReleaseRule } from "../../../generated/api/StudentFeedbackReleaseRule";
import type { StudentFeedbackReleaseTiming } from "../../../generated/api/StudentFeedbackReleaseTiming";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";

import type {
  PolicyFocusTarget,
  AssignmentActivityRuleDraft,
  AssignmentActivityRuleDraftField,
} from "./assignment_workspace_policy_model";

function assignmentAttemptGradeRule(
  value: string,
): AssignmentActivityRules["assignmentAttemptGradeRule"] {
  if (
    value === "first" ||
    value === "latest" ||
    value === "highest" ||
    value === "instructorSelected"
  ) {
    return value;
  }
  throw new Error("Assignment Attempt Grade Rule selection is invalid");
}

function questionPoolReuseRule(value: string): AssignmentActivityRules["questionPoolReuseRule"] {
  if (value === "reuseSelection" || value === "selectAgain") {
    return value;
  }
  throw new Error("Question Pool Reuse Rule selection is invalid");
}

function questionVariationRule(value: string): AssignmentActivityRules["questionVariationRule"] {
  if (value === "reuseVariation" || value === "newVariation") return value;
  throw new Error("Question Variation Rule selection is invalid");
}

function assignmentAttemptResumeRule(
  value: string,
): AssignmentActivityRules["assignmentAttemptResumeRule"] {
  if (value === "resumable" || value === "singleSession") return value;
  throw new Error("Assignment Attempt Resume Rule selection is invalid");
}

function assignmentQuestionDisplayRule(
  value: string,
): AssignmentActivityRules["assignmentQuestionDisplayRule"] {
  if (value === "allQuestions" || value === "oneQuestionAtATime") return value;
  throw new Error("Assignment Question Display Rule selection is invalid");
}

function assignmentNavigationRule(
  value: string,
): AssignmentActivityRules["assignmentNavigationRule"] {
  if (value === "freeNavigation" || value === "forwardOnly") return value;
  throw new Error("Assignment Navigation Rule selection is invalid");
}

function assignmentQuestionOrderRule(
  value: string,
): AssignmentActivityRules["assignmentQuestionOrderRule"] {
  if (value === "authoredOrder" || value === "shuffled") return value;
  throw new Error("Assignment Question Order Rule selection is invalid");
}

function studentFeedbackReleaseTiming(value: string): StudentFeedbackReleaseTiming {
  if (
    value === "during_attempt" ||
    value === "after_submit" ||
    value === "after_due" ||
    value === "after_close" ||
    value === "never"
  ) {
    return value;
  }
  throw new Error("Student Feedback Release timing selection is invalid");
}

const studentFeedbackReleaseTimingOptions: ReadonlyArray<
  readonly [StudentFeedbackReleaseTiming, string]
> = [
  ["during_attempt", "While they work"],
  ["after_submit", "After they submit"],
  ["after_due", "After the due time"],
  ["after_close", "After the close time"],
  ["never", "Never"],
];

interface AssignmentWorkspacePolicyPanelProps {
  readonly policies: () => AssignmentActivityRules;
  readonly studentFeedbackReleaseRule: () => StudentFeedbackReleaseRule;
  readonly activityRuleDraft: () => AssignmentActivityRuleDraft;
  readonly activityRuleFieldError: (field: AssignmentActivityRuleDraftField) => string | undefined;
  readonly questionPoolReuseRuleError: () => string | undefined;
  readonly questionVariationRuleError: () => string | undefined;
  readonly onPoliciesChange: (policies: AssignmentActivityRules) => void;
  readonly onQuestionPoolReuseRuleChange: (policies: AssignmentActivityRules) => void;
  readonly onQuestionVariationRuleChange: (policies: AssignmentActivityRules) => void;
  readonly onStudentFeedbackReleaseRuleChange: (rule: StudentFeedbackReleaseRule) => void;
  readonly onActivityRuleDraftChange: (
    field: AssignmentActivityRuleDraftField,
    raw: string,
  ) => void;
  readonly onCompletionKindChange: (
    kind: AssignmentActivityRules["assignmentCompletionRule"]["kind"],
  ) => void;
  readonly onAssignmentAttemptContinuationRuleKindChange: (
    kind: AssignmentActivityRules["assignmentAttemptContinuationRule"]["kind"],
  ) => void;
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
  function changeDisclosure(field: keyof StudentFeedbackReleaseRule, value: string): void {
    props.onStudentFeedbackReleaseRuleChange({
      ...props.studentFeedbackReleaseRule(),
      [field]: studentFeedbackReleaseTiming(value),
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
            value={props.policies().assignmentCompletionRule.kind}
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
        <Show when={props.policies().assignmentCompletionRule.kind === "scoreAtLeast"}>
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
        <legend>Assignment Attempt grade rule</legend>
        <label class="assignment-editor-field">
          Record
          <select
            aria-label="Assignment Attempt grade rule"
            value={props.policies().assignmentAttemptGradeRule}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                assignmentAttemptGradeRule: assignmentAttemptGradeRule(event.currentTarget.value),
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
        <legend>Assignment Attempt continuation rule</legend>
        <label class="assignment-editor-field">
          After completion
          <select
            aria-label="Assignment Attempt continuation rule"
            value={props.policies().assignmentAttemptContinuationRule.kind}
            onChange={(event) => {
              const kind = event.currentTarget.value;
              if (kind === "closed" || kind === "capped" || kind === "unlimited") {
                props.onAssignmentAttemptContinuationRuleKindChange(kind);
              }
            }}
          >
            <option value="unlimited">Allow unlimited practice</option>
            <option value="capped">Limit additional Assignment Attempts</option>
            <option value="closed">Close after completion</option>
          </select>
        </label>
        <Show when={props.policies().assignmentAttemptContinuationRule.kind === "capped"}>
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
        <legend>Later Assignment Attempt rules</legend>
        <label class="assignment-editor-field">
          Question Pool selection
          <select
            aria-label="Question Pool Reuse Rule"
            ref={(element) => props.onRegisterPolicyControl("questionPoolReuseRule", element)}
            value={props.policies().questionPoolReuseRule}
            aria-invalid={props.questionPoolReuseRuleError() !== undefined}
            aria-describedby={
              props.questionPoolReuseRuleError() === undefined
                ? undefined
                : "assignment-policies-field-error"
            }
            onChange={(event) =>
              props.onQuestionPoolReuseRuleChange({
                ...props.policies(),
                questionPoolReuseRule: questionPoolReuseRule(event.currentTarget.value),
              })
            }
          >
            <option value="reuseSelection">Reuse the previous Question Pool Selection</option>
            <option value="selectAgain">Select Questions again from each Question Pool</option>
          </select>
        </label>
        <label class="assignment-editor-field">
          Question Variation
          <select
            aria-label="Question variation rule"
            ref={(element) => props.onRegisterPolicyControl("questionVariationRule", element)}
            value={props.policies().questionVariationRule}
            aria-invalid={props.questionVariationRuleError() !== undefined}
            aria-describedby={
              props.questionVariationRuleError() === undefined
                ? undefined
                : "assignment-policies-field-error"
            }
            onChange={(event) =>
              props.onQuestionVariationRuleChange({
                ...props.policies(),
                questionVariationRule: questionVariationRule(event.currentTarget.value),
              })
            }
          >
            <option value="reuseVariation">Reuse the previous Question Variations</option>
            <option value="newVariation">Use new Question Variations</option>
          </select>
          <Show when={props.questionVariationRuleError()}>
            {(message) => (
              <p class="assignment-editor-note" role="status">
                {message()}
              </p>
            )}
          </Show>
        </label>
      </fieldset>
      <fieldset class="assignment-editor-policy-set assignment-editor-policy-set--delivery">
        <legend>Assignment Attempt delivery rules</legend>
        <label class="assignment-editor-field">
          Resume
          <select
            aria-label="Assignment Attempt Resume Rule"
            value={props.policies().assignmentAttemptResumeRule}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                assignmentAttemptResumeRule: assignmentAttemptResumeRule(event.currentTarget.value),
              })
            }
          >
            <option value="resumable">Students may leave and resume</option>
            <option value="singleSession">Students complete one active session</option>
          </select>
        </label>
        <label class="assignment-editor-field">
          Question display
          <select
            aria-label="Assignment Question Display Rule"
            value={props.policies().assignmentQuestionDisplayRule}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                assignmentQuestionDisplayRule: assignmentQuestionDisplayRule(
                  event.currentTarget.value,
                ),
              })
            }
          >
            <option value="allQuestions">Show all Questions</option>
            <option value="oneQuestionAtATime">Show one Question at a time</option>
          </select>
        </label>
        <label class="assignment-editor-field">
          Navigation
          <select
            aria-label="Assignment Navigation Rule"
            value={props.policies().assignmentNavigationRule}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                assignmentNavigationRule: assignmentNavigationRule(event.currentTarget.value),
              })
            }
          >
            <option value="freeNavigation">Students may revisit Questions</option>
            <option value="forwardOnly">Students move forward only</option>
          </select>
        </label>
        <label class="assignment-editor-field">
          Question order
          <select
            aria-label="Assignment Question Order Rule"
            value={props.policies().assignmentQuestionOrderRule}
            onChange={(event) =>
              props.onPoliciesChange({
                ...props.policies(),
                assignmentQuestionOrderRule: assignmentQuestionOrderRule(event.currentTarget.value),
              })
            }
          >
            <option value="authoredOrder">Keep authored Question order</option>
            <option value="shuffled">Shuffle Questions for each Assignment Attempt</option>
          </select>
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
          value={props.studentFeedbackReleaseRule().score}
          onChange={(value) => changeDisclosure("score", value)}
        />
        <DisclosureControl
          label="Per-item correctness"
          value={props.studentFeedbackReleaseRule().per_item_correctness}
          onChange={(value) => changeDisclosure("per_item_correctness", value)}
        />
        <DisclosureControl
          label="Feedback text"
          value={props.studentFeedbackReleaseRule().feedback_text}
          onChange={(value) => changeDisclosure("feedback_text", value)}
        />
        <DisclosureControl
          label="Show Question Answer"
          value={props.studentFeedbackReleaseRule().question_answer}
          onChange={(value) => changeDisclosure("question_answer", value)}
        />
        <DisclosureControl
          label="Show Explanation"
          value={props.studentFeedbackReleaseRule().question_answer_explanation}
          onChange={(value) => changeDisclosure("question_answer_explanation", value)}
        />
        <DisclosureControl
          label="Class statistics"
          value={props.studentFeedbackReleaseRule().class_statistics}
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
  readonly value: StudentFeedbackReleaseTiming;
  readonly onChange: (value: string) => void;
}): JSX.Element {
  return (
    <label class="assignment-editor-field">
      {props.label}
      <select value={props.value} onChange={(event) => props.onChange(event.currentTarget.value)}>
        {studentFeedbackReleaseTimingOptions.map(([value, label]) => (
          <option value={value}>{label}</option>
        ))}
      </select>
    </label>
  );
}
