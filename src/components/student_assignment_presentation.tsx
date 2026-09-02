// student_assignment_presentation.tsx - answer-free assignment landing surface.

import { Show, type JSX } from "solid-js";

import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { StudentAssignmentDetail } from "../../generated/api/StudentAssignmentDetail";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { StudentClassStatistics } from "../../generated/api/StudentClassStatistics";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { StudentFeedbackReleaseTiming } from "../../generated/api/StudentFeedbackReleaseTiming";
import type { StudentLateWorkStatus } from "../../generated/api/StudentLateWorkStatus";
import type { QuestionPoolReuseRule } from "../../generated/api/QuestionPoolReuseRule";
import type { AssignmentQuestionVariationRule } from "../../generated/api/AssignmentQuestionVariationRule";
import { studentProgressSummary, studentScoreValue } from "../student_progress";

export interface StudentAssignmentPresentationDelivery {
  readonly availableAt: number | null;
  readonly dueAt: number | null;
  readonly closesAt: number | null;
  readonly assignmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly lateWorkRule: "accept" | "markLate" | "reject";
  readonly assignmentDeadlineRule: "autoSubmit";
  readonly lateStatus?: StudentLateWorkStatus;
}

/**
 * The answer-free data needed to render an assignment landing surface.
 *
 * This is deliberately independent of route, session, and run state. The
 * Instructor Student-view projection can provide the same shape without
 * introducing student identity or mutation capabilities.
 */
export interface StudentAssignmentPresentationData {
  readonly title: string;
  readonly instructions: string;
  readonly timeZone: string;
  readonly delivery: StudentAssignmentPresentationDelivery;
  readonly questionsPerRun: number;
  readonly questionPoolReuseRule?: QuestionPoolReuseRule;
  readonly questionVariationRule?: AssignmentQuestionVariationRule;
  readonly studentFeedbackReleaseRule?: StudentFeedbackReleaseRule;
}

export interface StudentAssignmentPresentationProps {
  readonly assignment: StudentAssignmentPresentationData;
  readonly progress?: AssignmentProgress;
  readonly contextCue?: JSX.Element;
  readonly returnAction?: JSX.Element;
  readonly secondaryAction?: JSX.Element | null;
  readonly primaryAction: JSX.Element | null;
}

/** Adapts either answer-free server projection to the shared presentation shape. */
export function toStudentAssignmentPresentationData(
  assignment: StudentAssignmentDetail | InstructorStudentView,
): StudentAssignmentPresentationData {
  if ("questionsPerRun" in assignment) {
    return {
      title: assignment.title,
      instructions: assignment.instructions,
      timeZone: assignment.timeZone,
      delivery: assignment.delivery,
      questionsPerRun: assignment.questionsPerRun,
      questionPoolReuseRule: assignment.questionPoolReuseRule,
      questionVariationRule: assignment.questionVariationRule,
      studentFeedbackReleaseRule: assignment.studentFeedbackReleaseRule,
    };
  }

  return {
    title: assignment.title,
    instructions: assignment.instructions,
    timeZone: assignment.time_zone,
    delivery: {
      availableAt: assignment.delivery.available_at,
      dueAt: assignment.delivery.due_at,
      closesAt: assignment.delivery.closes_at,
      assignmentAttemptTimeLimitSeconds: assignment.delivery.assignment_attempt_time_limit_seconds,
      attemptLimit: assignment.delivery.attempt_limit,
      lateWorkRule: assignment.delivery.late_work_rule,
      assignmentDeadlineRule: assignment.delivery.assignment_deadline_rule,
      lateStatus: assignment.delivery.late_status,
    },
    questionsPerRun: assignment.entries.reduce(
      (count, entry) =>
        entry.kind === "fixedQuestion"
          ? count + (entry.availability === "available" ? 1 : 0)
          : count + (entry.availability === "available" ? entry.selectionCount : 0),
      0,
    ),
  };
}

export function formatAssignmentActivity(timestamp: number | null): string {
  if (timestamp === null) {
    return "No activity yet";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}

export function formatAssignmentDeliveryTime(timestamp: number | null, timeZone: string): string {
  if (timestamp === null) return "Not set";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(timestamp));
}

export function formatAssignmentLimit(
  value: number | null,
  singular: string,
  plural: string,
): string {
  if (value === null) return `No ${plural} limit`;
  return `${value} ${value === 1 ? singular : plural}`;
}

export function formatAssignmentAttemptTimeLimit(seconds: number | null): string {
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

export function formatLateWorkRule(value: "accept" | "markLate" | "reject"): string {
  if (value === "accept") return "Accepted after the due time";
  if (value === "markLate") return "Accepted and marked late after the due time";
  return "Not accepted after the due time";
}

function formatLateStatus(value: StudentLateWorkStatus): string {
  if (value === "on_time") return "On time";
  if (value === "marked_late") return "Accepted and marked late";
  return "Accepted late";
}

function formatLaterAttemptRules(
  poolReuseRule: QuestionPoolReuseRule | undefined,
  variationRule: AssignmentQuestionVariationRule | undefined,
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
  return `A later Assignment Attempt ${selection} and ${variation}.`;
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
  const feedbackTiming = formatDisclosureTiming(rule.feedback_text);
  const questionAnswerTiming = formatDisclosureTiming(rule.question_answer);
  const questionAnswerExplanationTiming = formatDisclosureTiming(rule.question_answer_explanation);
  return `Question Feedback is shown ${feedbackTiming}; Question Answer is shown ${questionAnswerTiming}; Answer Explanation is shown ${questionAnswerExplanationTiming}.`;
}

function classStatisticsSummary(statistics: StudentClassStatistics): string {
  if (statistics.state === "insufficient_evidence") {
    return "Not enough evidence to show class statistics.";
  }
  return `Class average: ${studentScoreValue(
    statistics.assignment_average_score,
  )}. Based on ${statistics.completed_student_cohort_size} completed students.`;
}

export function StudentAssignmentPresentation(
  props: StudentAssignmentPresentationProps,
): JSX.Element {
  function disclosure(): string | undefined {
    return disclosureSummary(props.assignment.studentFeedbackReleaseRule);
  }

  function hasActions(): boolean {
    return (
      (props.primaryAction !== null && props.primaryAction !== undefined) ||
      (props.secondaryAction !== null && props.secondaryAction !== undefined)
    );
  }

  return (
    <div class="student-assignment-presentation">
      <Show when={props.contextCue}>
        <div class="student-assignment-context empty-state" role="note">
          {props.contextCue}
        </div>
      </Show>
      <Show when={props.returnAction}>
        <div class="student-assignment-return">{props.returnAction}</div>
      </Show>
      <p class="eyebrow">Assignment overview</p>
      <h1>{props.assignment.title}</h1>
      <Show when={hasActions()}>
        <div class="student-assignment-action-region" role="group" aria-label="Practice actions">
          <Show when={props.primaryAction}>
            <div class="student-assignment-primary-action">{props.primaryAction}</div>
          </Show>
          <Show when={props.secondaryAction}>
            <div class="student-assignment-secondary-actions">{props.secondaryAction}</div>
          </Show>
        </div>
      </Show>
      <p class="page-lede">
        Work from the structures and concepts in front of you. Memorization is not the goal.
      </p>
      <Show when={props.assignment.instructions.length > 0}>
        <section aria-labelledby="assignment-instructions-heading">
          <h2 id="assignment-instructions-heading">Instructions</h2>
          <p class="plain-text-instructions">{props.assignment.instructions}</p>
        </section>
      </Show>
      <section aria-labelledby="delivery-details-heading">
        <h2 id="delivery-details-heading">Delivery details</h2>
        <p>Times are shown in the course time zone: {props.assignment.timeZone}.</p>
        <dl class="assignment-facts">
          <div>
            <dt>Available</dt>
            <dd>
              {formatAssignmentDeliveryTime(
                props.assignment.delivery.availableAt,
                props.assignment.timeZone,
              )}
            </dd>
          </div>
          <div>
            <dt>Due</dt>
            <dd>
              {formatAssignmentDeliveryTime(
                props.assignment.delivery.dueAt,
                props.assignment.timeZone,
              )}
            </dd>
          </div>
          <div>
            <dt>Closes</dt>
            <dd>
              {formatAssignmentDeliveryTime(
                props.assignment.delivery.closesAt,
                props.assignment.timeZone,
              )}
            </dd>
          </div>
          <div>
            <dt>Whole-run limit</dt>
            <dd>
              {formatAssignmentAttemptTimeLimit(
                props.assignment.delivery.assignmentAttemptTimeLimitSeconds,
              )}
            </dd>
          </div>
          <div>
            <dt>Attempt limit</dt>
            <dd>
              {formatAssignmentLimit(props.assignment.delivery.attemptLimit, "attempt", "attempts")}
            </dd>
          </div>
          <div>
            <dt>Late work</dt>
            <dd>{formatLateWorkRule(props.assignment.delivery.lateWorkRule)}</dd>
          </div>
          <div>
            <dt>Deadline behavior</dt>
            <dd>The server automatically submits work at its effective deadline.</dd>
          </div>
          <Show when={props.assignment.delivery.lateStatus}>
            {(lateStatus) => (
              <div>
                <dt>Late status</dt>
                <dd>{formatLateStatus(lateStatus())}</dd>
              </div>
            )}
          </Show>
        </dl>
      </section>
      <dl class="assignment-facts">
        <div>
          <dt>Questions per run</dt>
          <dd>{props.assignment.questionsPerRun}</dd>
        </div>
        <div>
          <dt>Later Assignment Attempt</dt>
          <dd>
            {formatLaterAttemptRules(
              props.assignment.questionPoolReuseRule,
              props.assignment.questionVariationRule,
            )}
          </dd>
        </div>
        <div>
          <dt>Student Feedback Release</dt>
          <dd>
            {disclosure() ?? "Student Feedback is available according to the Assignment settings."}
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
                  progress().score_state === "available" &&
                  progress().assignment_scoring_state === "current"
                }
              >
                <div>
                  <dt>Current score</dt>
                  <dd>{studentScoreValue(progress().current_score)}</dd>
                </div>
                <div>
                  <dt>Latest score</dt>
                  <dd>{studentScoreValue(progress().latest_score)}</dd>
                </div>
                <div>
                  <dt>Best score</dt>
                  <dd>{studentScoreValue(progress().best_score)}</dd>
                </div>
              </Show>
              <div>
                <dt>Completed runs</dt>
                <dd>{progress().completed_assignment_attempt_count}</dd>
              </div>
              <div>
                <dt>Total attempts</dt>
                <dd>{progress().total_question_attempts}</dd>
              </div>
              <div>
                <dt>Last activity</dt>
                <dd>{formatAssignmentActivity(progress().last_activity_at)}</dd>
              </div>
              <Show when={progress().class_statistics}>
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
