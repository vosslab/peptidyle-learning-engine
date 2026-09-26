import { For, type JSX } from "solid-js";
import type {
  AssessmentBlueprintUpdateContent,
  AssessmentBlueprintUpdateEntry,
  LiveAssessmentWorkspace,
} from "../../api/assessment_release";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { AssessmentEntryAvailability } from "../../../generated/api/AssessmentEntryAvailability";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import {
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
} from "../../assessment_duration";
import { RecordSequence } from "../../components/record_list/record_sequence";
import type { RecordContent } from "../../components/record_list/record_list";

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

/** Shared semantic presentation for each current or proposed ordered Assessment entry. */
function assessmentEntryContent(props: {
  readonly entry: AssessmentBlueprintUpdateEntry & {
    readonly availability?: AssessmentEntryAvailability;
  };
  readonly description: (publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple) => string;
  readonly poolRole: "assessmentOwned" | "librarySource";
}): RecordContent {
  const timeLimit = (): string =>
    props.entry.questionAttemptTimeLimit.kind === "unlimited"
      ? "No Question time limit"
      : `${props.entry.questionAttemptTimeLimit.seconds}s Question time limit; ${props.entry.questionAttemptTimeLimit.graceSeconds}s grace`;
  const commonDetails: ReadonlyArray<RecordContent["details"][number]> = [
    ...(props.entry.availability === undefined
      ? []
      : [{ kind: "text" as const, label: "Availability", value: props.entry.availability }]),
    { kind: "text", label: "Scoring", value: `${props.entry.scoringRule} scoring` },
    {
      kind: "text",
      label: "Question Attempts",
      value: `${props.entry.questionAttemptLimit.maxAttempts ?? "Unlimited"}`,
    },
    { kind: "text", label: "Question time limit", value: timeLimit() },
  ];

  if (props.entry.kind === "fixedQuestion") {
    const revision = props.entry.publishedQuestionRevisionTuple;
    return {
      title: `Question ${revision.publishedQuestionId}, revision ${revision.revisionNumber}`,
      description: props.description(revision),
      details: [
        { kind: "text", label: "Points", value: `${props.entry.pointsPossible} points` },
        ...commonDetails,
      ],
      actions: [],
    };
  }

  const poolLabel =
    props.poolRole === "assessmentOwned"
      ? "Assessment-owned Question Pool"
      : "Library source Question Pool";
  return {
    title: `${poolLabel} ${props.entry.questionPoolId}, edit ${props.entry.questionPoolEditNumber}`,
    details: [
      { kind: "text", label: "Selection count", value: `${props.entry.selectionCount}` },
      {
        kind: "text",
        label: "Points per Question",
        value: `${props.entry.pointsPerItem} points`,
      },
      {
        kind: "text",
        label: "Selected Question order",
        value:
          props.entry.selectionRule.selectedQuestionOrder === "randomOrder"
            ? "Random selected Question order"
            : "Question Pool order",
      },
      ...commonDetails,
    ],
    actions: [],
  };
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
  readonly description: (publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple) => string;
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
        <RecordSequence
          rows={props.content.entries.map((entry, index) => ({ entry, index }))}
          content={(record) =>
            assessmentEntryContent({
              entry: record.entry,
              description: props.description,
              poolRole: props.poolRole,
            })
          }
          recordId={(record) => `${record.entry.kind}:${record.index}`}
          state={{ kind: "ready" }}
          ariaLabel={`${props.heading} ordered Questions and Question Pools`}
          emptyState={{ title: "No Entries." }}
        />
      )}
    </section>
  );
}
