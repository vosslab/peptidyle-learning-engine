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
          Time limit in seconds (optional)
          <input
            type="number"
            min="1"
            step="1"
            inputmode="numeric"
            value={props.draft.timeLimit}
            onInput={(event) => props.onPatch({ timeLimit: event.currentTarget.value })}
          />
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
          Gradebook Assessment Attempt
          <select
            value={props.draft.gradeRule}
            onChange={(event) =>
              props.onPatch({
                gradeRule: event.currentTarget.value as AssessmentTemplateDraft["gradeRule"],
              })
            }
          >
            <option value="first">First</option>
            <option value="latest">Latest</option>
            <option value="highest">Highest</option>
            <option value="instructorSelected">Instructor selected</option>
          </select>
        </label>
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
          Leaving an Assessment Attempt
          <select
            value={props.draft.resumeRule}
            onChange={(event) =>
              props.onPatch({
                resumeRule: event.currentTarget.value as AssessmentTemplateDraft["resumeRule"],
              })
            }
          >
            <option value="resumable">May resume later</option>
            <option value="singleSession">Single session</option>
          </select>
        </label>
        <label class="assessment-template-field">
          Question display
          <select
            value={props.draft.displayRule}
            onChange={(event) =>
              props.onPatch({
                displayRule: event.currentTarget.value as AssessmentTemplateDraft["displayRule"],
              })
            }
          >
            <option value="allQuestions">All Questions</option>
            <option value="oneQuestionAtATime">One Question at a time</option>
          </select>
        </label>
        <label class="assessment-template-field">
          Navigation
          <select
            value={props.draft.navigationRule}
            onChange={(event) =>
              props.onPatch({
                navigationRule: event.currentTarget
                  .value as AssessmentTemplateDraft["navigationRule"],
              })
            }
          >
            <option value="freeNavigation">Free navigation</option>
            <option value="forwardOnly">Forward only</option>
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
