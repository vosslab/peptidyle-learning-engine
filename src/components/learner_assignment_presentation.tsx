// learner_assignment_presentation.tsx - answer-free assignment landing surface.

import { Show, type JSX } from "solid-js";

import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { LearnerAssignmentDetail } from "../../generated/api/LearnerAssignmentDetail";
import type { LearnerAssignmentProgress } from "../../generated/api/LearnerAssignmentProgress";
import type { LearnerClassStatistics } from "../../generated/api/LearnerClassStatistics";
import type { LearnerDisclosurePolicy } from "../../generated/api/LearnerDisclosurePolicy";
import type { LearnerDisclosureTiming } from "../../generated/api/LearnerDisclosureTiming";
import type { LearnerLateStatus } from "../../generated/api/LearnerLateStatus";
import type { VariationPolicy } from "../../generated/api/VariationPolicy";
import { learnerProgressSummary, learnerScoreValue } from "../learner_progress";

export interface LearnerAssignmentPresentationDelivery {
  readonly availableAt: number | null;
  readonly dueAt: number | null;
  readonly closesAt: number | null;
  readonly timeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly lateSubmission: "accept" | "markLate" | "reject";
  readonly deadlineBehavior: "autoSubmit";
  readonly lateStatus?: LearnerLateStatus;
}

/**
 * The answer-free data needed to render an assignment landing surface.
 *
 * This is deliberately independent of route, session, and run state. The
 * Instructor Student-view projection can provide the same shape without
 * introducing learner identity or mutation capabilities.
 */
export interface LearnerAssignmentPresentationData {
  readonly title: string;
  readonly instructions: string;
  readonly timeZone: string;
  readonly delivery: LearnerAssignmentPresentationDelivery;
  readonly questionsPerRun: number;
  readonly variation?: VariationPolicy;
  readonly disclosurePolicy?: LearnerDisclosurePolicy;
}

export interface LearnerAssignmentPresentationProps {
  readonly assignment: LearnerAssignmentPresentationData;
  readonly progress?: LearnerAssignmentProgress;
  readonly contextCue?: JSX.Element;
  readonly returnAction?: JSX.Element;
  readonly secondaryAction?: JSX.Element | null;
  readonly primaryAction: JSX.Element | null;
}

/** Adapts either answer-free server projection to the shared presentation shape. */
export function toLearnerAssignmentPresentationData(
  assignment: LearnerAssignmentDetail | InstructorStudentView,
): LearnerAssignmentPresentationData {
  const questionsPerRun =
    "questionsPerRun" in assignment
      ? assignment.questionsPerRun
      : assignment.items.filter((item) => item.deliveryState === "active").length +
        assignment.selectionGroups.reduce((count, group) => count + group.drawCount, 0);
  const variation = "variation" in assignment ? assignment.variation : undefined;
  const disclosurePolicy =
    "disclosurePolicy" in assignment ? assignment.disclosurePolicy : undefined;

  return {
    title: assignment.title,
    instructions: assignment.instructions,
    timeZone: assignment.timeZone,
    delivery: assignment.delivery,
    questionsPerRun,
    variation,
    disclosurePolicy,
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

export function formatAssignmentRunTimeLimit(seconds: number | null): string {
  if (seconds === null) return "No whole-run time limit";
  if (seconds % 3_600 === 0) {
    const hours = seconds / 3_600;
    return `${hours} ${hours === 1 ? "hour" : "hours"} per run`;
  }
  if (seconds % 60 === 0) {
    const minutes = seconds / 60;
    return `${minutes} ${minutes === 1 ? "minute" : "minutes"} per run`;
  }
  return `${seconds} ${seconds === 1 ? "second" : "seconds"} per run`;
}

export function formatLateSubmission(value: "accept" | "markLate" | "reject"): string {
  if (value === "accept") return "Accepted after the due time";
  if (value === "markLate") return "Accepted and marked late after the due time";
  return "Not accepted after the due time";
}

function formatLateStatus(value: LearnerLateStatus): string {
  if (value === "onTime") return "On time";
  if (value === "markedLate") return "Accepted and marked late";
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

function formatDisclosureTiming(timing: LearnerDisclosureTiming): string {
  if (timing === "duringAttempt") return "during the attempt";
  if (timing === "afterSubmit") return "after submission";
  if (timing === "afterDue") return "after the due time";
  if (timing === "afterClose") return "after the close time";
  return "not shown";
}

function disclosureSummary(policy: LearnerDisclosurePolicy | undefined): string | undefined {
  if (policy === undefined) return undefined;
  const feedbackTiming = formatDisclosureTiming(policy.feedbackText);
  const solutionTiming = formatDisclosureTiming(policy.solution);
  return `Feedback is shown ${feedbackTiming}; solutions are shown ${solutionTiming}.`;
}

function classStatisticsSummary(statistics: LearnerClassStatistics): string {
  if (statistics.state === "insufficientEvidence") {
    return "Not enough evidence to show class statistics.";
  }
  return `Class average: ${learnerScoreValue(
    statistics.assignmentAverageScore,
  )}. Based on ${statistics.completedLearnerCohortSize} completed learners.`;
}

export function LearnerAssignmentPresentation(
  props: LearnerAssignmentPresentationProps,
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
    <div class="learner-assignment-presentation">
      <Show when={props.contextCue}>
        <div class="learner-assignment-context empty-state" role="note">
          {props.contextCue}
        </div>
      </Show>
      <Show when={props.returnAction}>
        <div class="learner-assignment-return">{props.returnAction}</div>
      </Show>
      <p class="eyebrow">Assignment overview</p>
      <h1>{props.assignment.title}</h1>
      <Show when={hasActions()}>
        <div class="learner-assignment-action-region" role="group" aria-label="Practice actions">
          <Show when={props.primaryAction}>
            <div class="learner-assignment-primary-action">{props.primaryAction}</div>
          </Show>
          <Show when={props.secondaryAction}>
            <div class="learner-assignment-secondary-actions">{props.secondaryAction}</div>
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
            <dd>{formatAssignmentRunTimeLimit(props.assignment.delivery.timeLimitSeconds)}</dd>
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
                <dd role="status">{learnerProgressSummary(progress())}</dd>
              </div>
              <Show
                when={
                  progress().scoreState === "available" && progress().scoringStatus === "current"
                }
              >
                <div>
                  <dt>Current score</dt>
                  <dd>{learnerScoreValue(progress().currentScore)}</dd>
                </div>
                <div>
                  <dt>Latest score</dt>
                  <dd>{learnerScoreValue(progress().latestScore)}</dd>
                </div>
                <div>
                  <dt>Best score</dt>
                  <dd>{learnerScoreValue(progress().bestScore)}</dd>
                </div>
              </Show>
              <div>
                <dt>Completed runs</dt>
                <dd>{progress().completedRunCount}</dd>
              </div>
              <div>
                <dt>Total attempts</dt>
                <dd>{progress().totalQuestionAttempts}</dd>
              </div>
              <div>
                <dt>Last activity</dt>
                <dd>{formatAssignmentActivity(progress().lastActivityAt)}</dd>
              </div>
              <Show when={progress().classStatistics}>
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
