// student_assessment_presentation.tsx - answer-free assessment landing surface.

import { Show, type JSX } from "solid-js";

import type { StudentAssessmentDetail } from "../../generated/api/StudentAssessmentDetail";
import type { StudentAssessmentProgress } from "../../generated/api/StudentAssessmentProgress";
import type { ClassStatistics } from "../../generated/api/ClassStatistics";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { StudentFeedbackReleaseTiming } from "../../generated/api/StudentFeedbackReleaseTiming";
import type { StudentLateWorkStatus } from "../../generated/api/StudentLateWorkStatus";
import type { QuestionPoolReuseRule } from "../../generated/api/QuestionPoolReuseRule";
import type { AssessmentQuestionVariationRule } from "../../generated/api/AssessmentQuestionVariationRule";
import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";
import { studentProgressSummary, studentScoreValue } from "../student_progress";

export interface StudentAssessmentPresentationDelivery {
  readonly availableAt: number | null;
  readonly dueAt: number | null;
  readonly closesAt: number | null;
  readonly assessmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly lateWorkRule: "accept" | "mark_late" | "reject";
  readonly studentLateWorkStatus?: StudentLateWorkStatus;
}

/**
 * The answer-free data needed to render an assessment landing surface.
 *
 * This is deliberately independent of route, session, and Assessment Attempt state. The
 * Instructor Student View can provide the same shape without
 * introducing student identity or mutation capabilities.
 */
export interface StudentAssessmentPresentationData {
  readonly title: string;
  readonly instructions: string;
  /** Authenticated viewer's server-provided IANA display zone. */
  readonly displayTimeZone: string;
  readonly delivery: StudentAssessmentPresentationDelivery;
  readonly questionsPerAssessmentAttempt: number;
  readonly questionPoolReuseRule?: QuestionPoolReuseRule;
  readonly questionVariationRule?: AssessmentQuestionVariationRule;
  readonly studentFeedbackReleaseRule?: StudentFeedbackReleaseRule;
}

export interface StudentAssessmentPresentationProps {
  readonly assessment: StudentAssessmentPresentationData;
  readonly progress?: StudentAssessmentProgress;
  readonly contextCue?: JSX.Element;
  readonly returnAction?: JSX.Element;
  readonly secondaryAction?: JSX.Element | null;
  readonly primaryAction: JSX.Element | null;
}

/** Compact, answer-free facts shared by the preview and Student Start surfaces. */
export function StudentAssessmentStartFacts(props: {
  readonly questionCount: number;
  readonly pointsPossible?: number;
  readonly timeLimitSeconds: number | null;
  readonly decision?: StudentAssessmentDecisionSummary;
}): JSX.Element {
  return (
    <section
      class="student-assessment-start-facts"
      aria-labelledby="assessment-start-facts-heading"
    >
      <h2 id="assessment-start-facts-heading">Before you start</h2>
      <dl class="assessment-facts">
        <div>
          <dt>Questions</dt>
          <dd>{props.questionCount}</dd>
        </div>
        <Show when={props.pointsPossible !== undefined}>
          <div>
            <dt>Points possible</dt>
            <dd>{props.pointsPossible}</dd>
          </div>
        </Show>
        <Show when={props.decision === undefined}>
          <div>
            <dt>Time limit</dt>
            <dd>{formatAssessmentAttemptTimeLimit(props.timeLimitSeconds)}</dd>
          </div>
        </Show>
      </dl>
      <Show when={props.decision}>
        {(decision) => <StudentAssessmentDecisionDetails decision={decision()} />}
      </Show>
    </section>
  );
}

/** Formats server-owned Assessment access facts without recomputing permission. */
export function StudentAssessmentDecisionDetails(props: {
  readonly decision: StudentAssessmentDecisionSummary;
}): JSX.Element {
  return (
    <div class="student-assessment-decision">
      <p class="student-assessment-decision__status" role="status">
        <strong>
          {props.decision.startDecision === "may_start" ? "Can start" : "Cannot start"}
        </strong>
        <Show when={props.decision.publicReason}>{(publicReason) => <> - {publicReason()}</>}</Show>
      </p>
      <p class="student-assessment-decision__zone">
        Times are shown in your time zone: {props.decision.displayTimeZone}.
      </p>
      <dl class="assessment-facts">
        <div>
          <dt>Available</dt>
          <dd>
            {formatAssessmentDeliveryTime(
              props.decision.availableAt,
              props.decision.displayTimeZone,
            )}
          </dd>
        </div>
        <div>
          <dt>Due</dt>
          <dd data-assessment-decision-due>
            {formatAssessmentDeliveryTime(props.decision.dueAt, props.decision.displayTimeZone)}
          </dd>
        </div>
        <div>
          <dt>Closes</dt>
          <dd>
            {formatAssessmentDeliveryTime(props.decision.closesAt, props.decision.displayTimeZone)}
          </dd>
        </div>
        <div>
          <dt>Time limit</dt>
          <dd>{formatAssessmentAttemptTimeLimit(props.decision.timeLimitSeconds)}</dd>
        </div>
        <div>
          <dt>Attempt limit</dt>
          <dd>{formatAssessmentLimit(props.decision.attemptLimit, "attempt", "attempts")}</dd>
        </div>
        <div>
          <dt>Late work</dt>
          <dd>{formatLateWorkRule(props.decision.lateWorkRule)}</dd>
        </div>
      </dl>
    </div>
  );
}

/** Adapts either answer-free Assessment Overview to the shared presentation shape. */
export function toStudentAssessmentPresentationData(
  assessment: StudentAssessmentDetail,
): StudentAssessmentPresentationData {
  return {
    title: assessment.title,
    instructions: assessment.instructions,
    displayTimeZone: assessment.display_time_zone,
    delivery: {
      availableAt: assessment.delivery.available_at,
      dueAt: assessment.delivery.due_at,
      closesAt: assessment.delivery.closes_at,
      assessmentAttemptTimeLimitSeconds: assessment.delivery.assessment_attempt_time_limit_seconds,
      attemptLimit: assessment.delivery.attempt_limit,
      lateWorkRule: assessment.delivery.late_work_rule,
      studentLateWorkStatus: assessment.delivery.student_late_work_status,
    },
    questionsPerAssessmentAttempt: assessment.entries.reduce(
      (count, entry) =>
        entry.kind === "fixedQuestion"
          ? count + (entry.availability === "available" ? 1 : 0)
          : count + (entry.availability === "available" ? entry.selectionCount : 0),
      0,
    ),
  };
}

export function formatAssessmentActivity(timestamp: number | null, timeZone: string): string {
  if (timestamp === null) {
    return "No activity yet";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(timestamp));
}

export function formatAssessmentDeliveryTime(timestamp: number | null, timeZone: string): string {
  if (timestamp === null) return "Not set";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(timestamp));
}

export function formatAssessmentLimit(
  value: number | null,
  singular: string,
  plural: string,
): string {
  if (value === null) return `No ${plural} limit`;
  return `${value} ${value === 1 ? singular : plural}`;
}

export function formatAssessmentAttemptTimeLimit(seconds: number | null): string {
  if (seconds === null) return "No whole-attempt time limit";
  if (seconds % 3_600 === 0) {
    const hours = seconds / 3_600;
    return `${hours} ${hours === 1 ? "hour" : "hours"} per attempt`;
  }
  if (seconds % 60 === 0) {
    const minutes = seconds / 60;
    return `${minutes} ${minutes === 1 ? "minute" : "minutes"} per attempt`;
  }
  return `${seconds} ${seconds === 1 ? "second" : "seconds"} per attempt`;
}

export function formatLateWorkRule(value: "accept" | "mark_late" | "reject"): string {
  if (value === "accept") return "Accepted after the due time";
  if (value === "mark_late") return "Accepted and marked late after the due time";
  return "Not accepted after the due time";
}

function formatStudentLateWorkStatus(value: StudentLateWorkStatus): string {
  if (value === "on_time") return "On time";
  if (value === "marked_late") return "Accepted and marked late";
  return "Accepted late";
}

function formatLaterAttemptRules(
  poolReuseRule: QuestionPoolReuseRule | undefined,
  variationRule: AssessmentQuestionVariationRule | undefined,
): string {
  const selection =
    poolReuseRule === "reuseSelection"
      ? "keeps its previous Question Pool Selection"
      : poolReuseRule === "selectAgain"
        ? "selects Questions again from each Question Pool"
        : "uses its Question Pool Reuse Rule";
  const variation =
    variationRule === "reuseVariation"
      ? "reuses the previous Question Variations"
      : variationRule === "newVariation"
        ? "uses new Question Variations"
        : "uses its Question Variation Rule";
  return `A later Assessment Attempt ${selection} and ${variation}.`;
}

function formatDisclosureTiming(timing: StudentFeedbackReleaseTiming): string {
  if (timing === "during_attempt") return "during the attempt";
  if (timing === "after_submit") return "after submission";
  if (timing === "after_due") return "after the due time";
  if (timing === "after_close") return "after the close time";
  return "not shown";
}

function disclosureSummary(rule: StudentFeedbackReleaseRule | undefined): string | undefined {
  if (rule === undefined) return undefined;
  const questionAnswerTiming = formatDisclosureTiming(rule.question_answer);
  const questionAnswerExplanationTiming = formatDisclosureTiming(rule.question_answer_explanation);
  return `Question Feedback is shown when provided; Question Answer is shown ${questionAnswerTiming}; Answer Explanation is shown ${questionAnswerExplanationTiming}.`;
}

function classStatisticsSummary(statistics: ClassStatistics): string {
  if (statistics.state === "unavailable") {
    return "Class Statistics are unavailable.";
  }
  return `Class average: ${studentScoreValue(
    statistics.assessment_average_score,
  )}. Based on ${statistics.completed_student_cohort_size} completed students.`;
}

export function StudentAssessmentPresentation(
  props: StudentAssessmentPresentationProps,
): JSX.Element {
  function disclosure(): string | undefined {
    return disclosureSummary(props.assessment.studentFeedbackReleaseRule);
  }

  function hasActions(): boolean {
    return (
      (props.primaryAction !== null && props.primaryAction !== undefined) ||
      (props.secondaryAction !== null && props.secondaryAction !== undefined)
    );
  }

  return (
    <div class="student-assessment-presentation">
      <Show when={props.contextCue}>
        <div class="student-assessment-context empty-state" role="note">
          {props.contextCue}
        </div>
      </Show>
      <Show when={props.returnAction}>
        <div class="student-assessment-return">{props.returnAction}</div>
      </Show>
      <p class="eyebrow">Assessment overview</p>
      <h1>{props.assessment.title}</h1>
      <Show when={hasActions()}>
        <div class="student-assessment-action-region" role="group" aria-label="Practice actions">
          <Show when={props.primaryAction}>
            <div class="student-assessment-primary-action">{props.primaryAction}</div>
          </Show>
          <Show when={props.secondaryAction}>
            <div class="student-assessment-secondary-actions">{props.secondaryAction}</div>
          </Show>
        </div>
      </Show>
      <p class="page-lede">
        Work from the structures and concepts in front of you. Memorization is not the goal.
      </p>
      <Show when={props.assessment.instructions.length > 0}>
        <section aria-labelledby="assessment-instructions-heading">
          <h2 id="assessment-instructions-heading">Instructions</h2>
          <p class="plain-text-instructions">{props.assessment.instructions}</p>
        </section>
      </Show>
      <StudentAssessmentStartFacts
        questionCount={props.assessment.questionsPerAssessmentAttempt}
        timeLimitSeconds={props.assessment.delivery.assessmentAttemptTimeLimitSeconds}
      />
      <section aria-labelledby="delivery-details-heading">
        <h2 id="delivery-details-heading">Delivery details</h2>
        <p>Times are shown in your time zone: {props.assessment.displayTimeZone}.</p>
        <dl class="assessment-facts">
          <div>
            <dt>Available</dt>
            <dd>
              {formatAssessmentDeliveryTime(
                props.assessment.delivery.availableAt,
                props.assessment.displayTimeZone,
              )}
            </dd>
          </div>
          <div>
            <dt>Due</dt>
            <dd>
              {formatAssessmentDeliveryTime(
                props.assessment.delivery.dueAt,
                props.assessment.displayTimeZone,
              )}
            </dd>
          </div>
          <div>
            <dt>Closes</dt>
            <dd>
              {formatAssessmentDeliveryTime(
                props.assessment.delivery.closesAt,
                props.assessment.displayTimeZone,
              )}
            </dd>
          </div>
          <div>
            <dt>Attempt limit</dt>
            <dd>
              {formatAssessmentLimit(props.assessment.delivery.attemptLimit, "attempt", "attempts")}
            </dd>
          </div>
          <div>
            <dt>Late work</dt>
            <dd>{formatLateWorkRule(props.assessment.delivery.lateWorkRule)}</dd>
          </div>
          <div>
            <dt>Deadline behavior</dt>
            <dd>The server automatically submits work at its effective deadline.</dd>
          </div>
          <Show when={props.assessment.delivery.studentLateWorkStatus}>
            {(studentLateWorkStatus) => (
              <div>
                <dt>Late work status</dt>
                <dd>{formatStudentLateWorkStatus(studentLateWorkStatus())}</dd>
              </div>
            )}
          </Show>
        </dl>
      </section>
      <dl class="assessment-facts">
        <div>
          <dt>Later Assessment Attempt</dt>
          <dd>
            {formatLaterAttemptRules(
              props.assessment.questionPoolReuseRule,
              props.assessment.questionVariationRule,
            )}
          </dd>
        </div>
        <div>
          <dt>Student Feedback Release</dt>
          <dd>
            {disclosure() ?? "Student Feedback is available according to the Assessment settings."}
          </dd>
        </div>
        <Show when={props.progress}>
          {(progress) => (
            <>
              <div>
                <dt>Score status</dt>
                <dd role="status">{studentProgressSummary(progress())}</dd>
              </div>
              <Show
                when={
                  progress().student_assessment_grade.score_state === "available" &&
                  progress().student_assessment_grade.assessment_scoring_state === "current"
                }
              >
                <div>
                  <dt>Current score</dt>
                  <dd>{studentScoreValue(progress().student_assessment_grade.current_score)}</dd>
                </div>
                <div>
                  <dt>Latest score</dt>
                  <dd>{studentScoreValue(progress().student_assessment_grade.latest_score)}</dd>
                </div>
                <div>
                  <dt>Best score</dt>
                  <dd>{studentScoreValue(progress().student_assessment_grade.best_score)}</dd>
                </div>
              </Show>
              <div>
                <dt>Completed Assessment Attempts</dt>
                <dd>{progress().assessment_progress.completed_assessment_attempt_count}</dd>
              </div>
              <div>
                <dt>Total attempts</dt>
                <dd>{progress().assessment_progress.total_question_attempts}</dd>
              </div>
              <div>
                <dt>Last activity</dt>
                <dd>
                  {formatAssessmentActivity(
                    progress().assessment_progress.last_activity_at,
                    props.assessment.displayTimeZone,
                  )}
                </dd>
              </div>
              <Show when={progress().student_assessment_grade.class_statistics}>
                {(statistics) => (
                  <div>
                    <dt>Class statistics</dt>
                    <dd>{classStatisticsSummary(statistics())}</dd>
                  </div>
                )}
              </Show>
            </>
          )}
        </Show>
      </dl>
    </div>
  );
}
