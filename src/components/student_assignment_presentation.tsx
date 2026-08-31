// student_assignment_presentation.tsx - answer-free assignment landing surface.

import { Show, type JSX } from "solid-js";

import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { StudentAssignmentDetail } from "../../generated/api/StudentAssignmentDetail";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { StudentClassStatistics } from "../../generated/api/StudentClassStatistics";
import type { StudentDisclosurePolicy } from "../../generated/api/StudentDisclosurePolicy";
import type { StudentDisclosureTiming } from "../../generated/api/StudentDisclosureTiming";
import type { StudentLateStatus } from "../../generated/api/StudentLateStatus";
import type { VariationPolicy } from "../../generated/api/VariationPolicy";
import { studentProgressSummary, studentScoreValue } from "../student_progress";

export interface StudentAssignmentPresentationDelivery {
  readonly availableAt: number | null;
  readonly dueAt: number | null;
  readonly closesAt: number | null;
  readonly timeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly lateSubmission: "accept" | "markLate" | "reject";
  readonly deadlineBehavior: "autoSubmit";
  readonly lateStatus?: StudentLateStatus;
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
  readonly variation?: VariationPolicy;
  readonly disclosurePolicy?: StudentDisclosurePolicy;
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
      variation: assignment.variation,
      disclosurePolicy: assignment.disclosurePolicy,
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
      timeLimitSeconds: assignment.delivery.time_limit_seconds,
      attemptLimit: assignment.delivery.attempt_limit,
      lateSubmission: assignment.delivery.late_submission,
      deadlineBehavior: assignment.delivery.deadline_behavior,
      lateStatus: assignment.delivery.late_status,
    },
    questionsPerRun: assignment.entries.reduce(
      (count, entry) =>
        entry.kind === "fixedQuestion"
          ? count + (entry.deliveryState === "active" ? 1 : 0)
          : count + entry.drawCount,
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

export function formatLateSubmission(value: "accept" | "markLate" | "reject"): string {
  if (value === "accept") return "Accepted after the due time";
  if (value === "markLate") return "Accepted and marked late after the due time";
  return "Not accepted after the due time";
}

function formatLateStatus(value: StudentLateStatus): string {
  if (value === "on_time") return "On time";
  if (value === "marked_late") return "Accepted and marked late";
  return "Accepted late";
}

function formatVariation(variation: VariationPolicy | undefined): string {
  if (variation === "newSeeds") return "Each attempt uses new question seeds.";
  if (variation === "selectedProblemVariants") {
    return "Each attempt selects problem variants.";
  }
  if (variation === "fullRegeneration") return "Each attempt is fully regenerated.";
  return "Each attempt is a fresh variation.";
}

function formatDisclosureTiming(timing: StudentDisclosureTiming): string {
  if (timing === "during_attempt") return "during the attempt";
  if (timing === "after_submit") return "after submission";
  if (timing === "after_due") return "after the due time";
  if (timing === "after_close") return "after the close time";
  return "not shown";
}

function disclosureSummary(policy: StudentDisclosurePolicy | undefined): string | undefined {
  if (policy === undefined) return undefined;
  const feedbackTiming = formatDisclosureTiming(policy.feedback_text);
  const solutionTiming = formatDisclosureTiming(policy.solution);
  return `Feedback is shown ${feedbackTiming}; solutions are shown ${solutionTiming}.`;
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
    return disclosureSummary(props.assignment.disclosurePolicy);
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
            <dd>{formatAssignmentAttemptTimeLimit(props.assignment.delivery.timeLimitSeconds)}</dd>
          </div>
          <div>
            <dt>Attempt limit</dt>
            <dd>
              {formatAssignmentLimit(props.assignment.delivery.attemptLimit, "attempt", "attempts")}
            </dd>
          </div>
          <div>
            <dt>Late work</dt>
            <dd>{formatLateSubmission(props.assignment.delivery.lateSubmission)}</dd>
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
          <dt>Variation</dt>
          <dd>{formatVariation(props.assignment.variation)}</dd>
        </div>
        <div>
          <dt>Feedback</dt>
          <dd>
            {disclosure() ??
              "Feedback and scores are available according to the assignment settings."}
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
                  progress().score_state === "available" && progress().scoring_status === "current"
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
