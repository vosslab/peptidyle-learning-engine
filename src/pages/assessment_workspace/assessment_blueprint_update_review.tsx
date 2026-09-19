import { For, type JSX } from "solid-js";
import type {
  AssessmentBlueprintUpdateContent,
  AssessmentBlueprintUpdateEntry,
  LiveAssessmentWorkspace,
} from "../../api/assessment_release";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import type { AssessmentEntryAvailability } from "../../../generated/api/AssessmentEntryAvailability";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import {
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
} from "../../assessment_duration";

/** Projects only reusable teaching content; Course delivery state stays outside the update. */
export function currentBlueprintUpdateContent(
  current: LiveAssessmentWorkspace,
): AssessmentBlueprintUpdateContent {
  return {
    assessmentType: current.assessmentType,
    title: current.title,
    instructions: current.instructions,
    defaults: {
      assessment_attempt_time_limit_seconds: current.assessmentAttemptTimeLimitSeconds,
      assessment_attempt_limit: current.attemptLimit,
      late_work_rule: current.lateWorkRule,
      activity_rules: current.activityRules,
      student_feedback_release_rule: current.studentFeedbackReleaseRule,
    },
    entries: current.entries,
  };
}

/** Shared by the Question Editor and both sides of a Blueprint update review. */
export function AssessmentEntrySummary(props: {
  readonly entry: AssessmentBlueprintUpdateEntry & {
    readonly availability?: AssessmentEntryAvailability;
  };
  readonly description: (questionRevision: QuestionRevisionTuple) => string;
  readonly poolRole?: "assessmentOwned" | "librarySource";
}): JSX.Element {
  const timeLimit = (): string =>
    props.entry.questionAttemptTimeLimit.kind === "unlimited"
      ? "No Question time limit"
      : `${props.entry.questionAttemptTimeLimit.seconds}s Question time limit; ${props.entry.questionAttemptTimeLimit.graceSeconds}s grace`;
  return (
    <>
      {props.entry.kind === "fixedQuestion" ? (
        <>
          <strong>{props.entry.questionRevision.questionId}</strong> * Revision{" "}
          {props.entry.questionRevision.revisionNumber}: {props.description(props.entry.questionRevision)};{" "}
          {props.entry.pointsPossible} points
        </>
      ) : (
        <>
          <strong>
            {props.poolRole === "assessmentOwned"
              ? "Assessment-owned Question Pool"
              : props.poolRole === "librarySource"
                ? "Library source Question Pool"
                : "Question Pool"}{" "}
            {props.entry.questionPoolId}
          </strong>{" "}
          * Edit {props.entry.questionPoolEditNumber}; select {props.entry.selectionCount};{" "}
          {props.entry.pointsPerItem} points per Question;{" "}
          {props.entry.selectionRule.selectedQuestionOrder === "randomOrder"
            ? "Random selected Question order"
            : "Question Pool order"}
        </>
      )}
      {props.entry.availability === undefined ? "" : ` (${props.entry.availability})`}
      {"; "}
      {props.entry.scoringRule} scoring;{" "}
      {props.entry.questionAttemptLimit.maxAttempts ?? "Unlimited"} Question Attempts; {timeLimit()}
    </>
  );
}

const ACTIVITY_LABELS = [
  ["questionVariationRule", "Question variation"],
  ["assessmentQuestionOrderRule", "Question order"],
] as const;
const FEEDBACK_LABELS = [
  ["score", "Score"],
  ["per_item_correctness", "Per-item correctness"],
  ["submitted_response", "Previous-attempt response"],
  ["question_answer", "Correct answer"],
  ["question_answer_explanation", "Question answer explanation"],
  ["class_statistics", "Class statistics"],
] as const;

function settingCopy(value: string): string {
  return value.replace(/_/gu, " ").replace(/([a-z])([A-Z])/gu, "$1 $2");
}

/** One complete labeled presentation, reused for current and proposed content without HTML interpretation. */
export function AssessmentBlueprintContentSummary(props: {
  readonly heading: string;
  readonly content: AssessmentBlueprintUpdateContent;
  readonly description: (questionRevision: QuestionRevisionTuple) => string;
  readonly poolRole: "assessmentOwned" | "librarySource";
}): JSX.Element {
  const defaults = (): AssessmentBlueprintUpdateContent["defaults"] => props.content.defaults;
  const assessmentDuration = (): string => {
    const seconds = defaults().assessment_attempt_time_limit_seconds;
    if (seconds !== null) return assessmentDurationDisplay(seconds);
    return assessmentDurationDefaultDescription(
      props.content.entries.reduce(
        (count, entry) => count + (entry.kind === "fixedQuestion" ? 1 : entry.selectionCount),
        0,
      ),
    );
  };
  // ASVS 1.2.1: titles, instructions, and Question descriptions remain escaped text nodes.
  return (
    <section class="assessment-editor-panel" aria-label={props.heading}>
      <h4>{props.heading}</h4>
      <dl class="assessment-facts">
        <div>
          <dt>Assessment Type</dt>
          <dd>{assessmentTypePresentation(props.content.assessmentType).label}</dd>
        </div>
        <div>
          <dt>Title</dt>
          <dd>{props.content.title}</dd>
        </div>
        <div>
          <dt>Student instructions</dt>
          <dd class="plain-text-instructions">{props.content.instructions || "No instructions"}</dd>
        </div>
        <div>
          <dt>Assessment Attempt time limit</dt>
          <dd>{assessmentDuration()}</dd>
        </div>
        <div>
          <dt>Assessment Attempt limit</dt>
          <dd>{defaults().assessment_attempt_limit ?? "Unlimited"}</dd>
        </div>
        <div>
          <dt>Late work</dt>
          <dd>{settingCopy(defaults().late_work_rule)}</dd>
        </div>
        <For each={ACTIVITY_LABELS}>
          {([field, label]) => (
            <div>
              <dt>{label}</dt>
              <dd>{settingCopy(defaults().activity_rules[field])}</dd>
            </div>
          )}
        </For>
        <For each={FEEDBACK_LABELS}>
          {([field, label]) => (
            <div>
              <dt>{label}</dt>
              <dd>{settingCopy(defaults().student_feedback_release_rule[field])}</dd>
            </div>
          )}
        </For>
      </dl>
      <h5>Ordered Questions and Question Pools</h5>
      {props.content.entries.length === 0 ? (
        <p>No Entries.</p>
      ) : (
        <ol>
          <For each={props.content.entries}>
            {(entry) => (
              <li>
                <AssessmentEntrySummary
                  entry={entry}
                  description={props.description}
                  poolRole={props.poolRole}
                />
              </li>
            )}
          </For>
        </ol>
      )}
    </section>
  );
}
