// assessment_template_settings_editor.tsx - complete reusable Assessment settings controls.

import { For, type JSX } from "solid-js";

import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { StudentFeedbackReleaseTiming } from "../../generated/api/StudentFeedbackReleaseTiming";
import type {
  AssessmentTemplateDraft,
  AssessmentTemplateDraftPatch,
} from "./assessment_template_settings_model";
import { assessmentTypeHasOneAttempt } from "./assessment_template_settings_model";

const FEEDBACK_FIELDS = [
  ["score", "Score"],
  ["per_item_correctness", "Per-item correctness"],
  ["submitted_response", "Submitted response"],
  ["question_answer", "Question Answer"],
  ["question_answer_explanation", "Question Answer Explanation"],
  ["class_statistics", "Class statistics"],
] as const;

const FEEDBACK_TIMINGS = [
  ["during_attempt", "During attempt"],
  ["after_submit", "After submit"],
  ["after_due", "After due"],
  ["after_close", "After close"],
  ["never", "Never"],
] as const satisfies ReadonlyArray<readonly [StudentFeedbackReleaseTiming, string]>;

export interface AssessmentTemplateSettingsEditorProps {
  readonly draft: AssessmentTemplateDraft;
  readonly disabled: boolean;
  readonly onPatch: (patch: AssessmentTemplateDraftPatch) => void;
}

function feedbackPatch(
  draft: AssessmentTemplateDraft,
  field: keyof StudentFeedbackReleaseRule,
  timing: StudentFeedbackReleaseTiming,
): AssessmentTemplateDraftPatch {
  return { feedback: { ...draft.feedback, [field]: timing } };
}

/** Groups every Template-owned setting without exposing Course delivery state. */
export function AssessmentTemplateSettingsEditor(
  props: AssessmentTemplateSettingsEditorProps,
): JSX.Element {
  return (
    <fieldset class="assessment-template-settings" disabled={props.disabled}>
      <legend>Reusable Assessment settings</legend>

      <section class="assessment-template-setting-group" aria-labelledby="template-basics-heading">
        <h3 id="template-basics-heading">Instructions and limits</h3>
        <label class="assessment-template-field assessment-template-field--wide">
          Student instructions
          <textarea
            rows="4"
            value={props.draft.instructions}
            onInput={(event) => props.onPatch({ instructions: event.currentTarget.value })}
          />
        </label>
        <label class="assessment-template-field">
          Assessment duration override in seconds (optional, maximum 12 hours)
          <input
            type="number"
            min="1"
            max="43200"
            step="1"
            inputmode="numeric"
            value={props.draft.timeLimit}
            onInput={(event) => props.onPatch({ timeLimit: event.currentTarget.value })}
          />
          <small>
            Default: 1.5 minutes per Question, rounded up to a whole minute. Templates have no
            Questions; the default is calculated after Questions are added to the Assessment. Leave
            the override blank to copy this default intent.
          </small>
        </label>
        <label class="assessment-template-field">
          Attempt limit (optional)
          <input
            type="number"
            min="1"
            step="1"
            inputmode="numeric"
            value={props.draft.attemptLimit}
            disabled={assessmentTypeHasOneAttempt(props.draft.assessmentType)}
            onInput={(event) => props.onPatch({ attemptLimit: event.currentTarget.value })}
          />
          {assessmentTypeHasOneAttempt(props.draft.assessmentType) ? (
            <small>Quiz and Exam permit exactly one Assessment Attempt.</small>
          ) : null}
        </label>
        <label class="assessment-template-field">
          Late-work rule
          <select
            value={props.draft.lateWorkRule}
            onChange={(event) =>
              props.onPatch({
                lateWorkRule: event.currentTarget.value as AssessmentTemplateDraft["lateWorkRule"],
              })
            }
          >
            <option value="reject">Reject late work</option>
            <option value="mark_late">Accept and mark late</option>
            <option value="accept">Accept late work</option>
          </select>
        </label>
      </section>

      <section
        class="assessment-template-setting-group"
        aria-labelledby="template-activity-heading"
      >
        <h3 id="template-activity-heading">Assessment activity</h3>
        <label class="assessment-template-field">
          Question variations on later Attempts
          <select
            value={props.draft.variationRule}
            onChange={(event) =>
              props.onPatch({
                variationRule: event.currentTarget
                  .value as AssessmentTemplateDraft["variationRule"],
              })
            }
          >
            <option value="reuseVariation">Reuse variations</option>
            <option value="newVariation">Use new variations</option>
          </select>
        </label>
        <label class="assessment-template-field">
          Question order
          <select
            value={props.draft.orderRule}
            onChange={(event) =>
              props.onPatch({
                orderRule: event.currentTarget.value as AssessmentTemplateDraft["orderRule"],
              })
            }
          >
            <option value="authoredOrder">Authored order</option>
            <option value="shuffled">Randomize Question order</option>
          </select>
        </label>
      </section>

      <section
        class="assessment-template-setting-group assessment-template-setting-group--feedback"
        aria-labelledby="template-feedback-heading"
      >
        <h3 id="template-feedback-heading">Student feedback</h3>
        <For each={FEEDBACK_FIELDS}>
          {([field, label]) => (
            <label class="assessment-template-field">
              {label}
              <select
                value={props.draft.feedback[field]}
                onChange={(event) =>
                  props.onPatch(
                    feedbackPatch(
                      props.draft,
                      field,
                      event.currentTarget.value as StudentFeedbackReleaseTiming,
                    ),
                  )
                }
              >
                <For each={FEEDBACK_TIMINGS}>
                  {([value, timingLabel]) => <option value={value}>{timingLabel}</option>}
                </For>
              </select>
            </label>
          )}
        </For>
      </section>
    </fieldset>
  );
}
