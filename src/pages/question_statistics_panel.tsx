// question_statistics_panel.tsx - explainable, answer-free Question Statistics and use detail.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionUseDetails } from "../../generated/api/QuestionUseDetails";
import { courseRouteReference } from "../navigation/public_route";
import "./question_statistics_panel.css";

export interface QuestionStatisticsPanelProps {
  readonly evidence: QuestionStatistics;
}

export interface QuestionUsePanelProps {
  readonly usage: QuestionUseDetails;
}

type AvailableQuestionStatistics = Extract<QuestionStatistics, { readonly state: "available" }>;
const wholeNumber = new Intl.NumberFormat("en-US");
const decimalNumber = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });
const percentage = new Intl.NumberFormat("en-US", { style: "percent", maximumFractionDigits: 1 });

function formatDuration(seconds: number): string {
  const roundedSeconds = Math.round(seconds);
  const minutes = Math.floor(roundedSeconds / 60);
  const remainingSeconds = roundedSeconds % 60;
  if (minutes === 0) return `${remainingSeconds} sec`;
  if (remainingSeconds === 0) return `${minutes} min`;
  return `${minutes} min ${remainingSeconds} sec`;
}

function formatCount(value: number, singular: string): string {
  return `${wholeNumber.format(value)} ${value === 1 ? singular : `${singular}s`}`;
}

/** Renders the server-owned, decomposed evidence without exposing a quality score. */
export function QuestionStatisticsPanel(props: QuestionStatisticsPanelProps): JSX.Element {
  const available = (): AvailableQuestionStatistics | undefined =>
    props.evidence.state === "available" ? props.evidence : undefined;

  return (
    <Show
      when={available()}
      fallback={
        <section class="catalog-statistics-panel" aria-labelledby="insufficient-evidence-heading">
          <h2 id="insufficient-evidence-heading">Learning evidence</h2>
          <p>
            More evidence is needed before shared learning measures can be shown. This question
            remains ranked by relevance, so you can still open it and decide whether it fits.
          </p>
        </section>
      }
    >
      {(evidence) => <AvailableEvidence evidence={evidence()} />}
    </Show>
  );
}

function AvailableEvidence(props: { readonly evidence: AvailableQuestionStatistics }): JSX.Element {
  const discrimination = (): { readonly value: number } | undefined => {
    const value = props.evidence.discriminationIndex;
    return value === undefined ? undefined : { value };
  };
  return (
    <section class="catalog-statistics-panel" aria-labelledby="learning-evidence-heading">
      <h2 id="learning-evidence-heading">Learning evidence</h2>
      <p class="catalog-statistics-introduction">
        These anonymous measures use independent Student observations for this exact published
        question. They pool observations across courses and describe association, not a cause or a
        prediction for your class.
      </p>
      <dl class="catalog-statistics-measures">
        <div>
          <dt>Observed courses</dt>
          <dd>{wholeNumber.format(props.evidence.observedCourseCount)} courses</dd>
        </div>
        <div>
          <dt>Independent Student observations</dt>
          <dd>
            {wholeNumber.format(props.evidence.independentLearnerObservationCount)} observations
          </dd>
        </div>
        <div>
          <dt>Difficulty (mean score)</dt>
          <dd>{percentage.format(props.evidence.difficultyIndex)}</dd>
        </div>
        <div>
          <dt>Mean submitted attempts</dt>
          <dd>{decimalNumber.format(props.evidence.attemptsMean)} attempts</dd>
        </div>
        <div>
          <dt>Estimated median duration</dt>
          <dd>{formatDuration(props.evidence.timeMedianSecondsEstimate)}</dd>
        </div>
        <Show when={discrimination()}>
          {(entry) => (
            <div>
              <dt>Discrimination</dt>
              <dd>{decimalNumber.format(entry().value)}</dd>
            </div>
          )}
        </Show>
      </dl>
    </section>
  );
}

/** Shows installation-wide counts and Account-authorized course links. */
export function QuestionUsePanel(props: QuestionUsePanelProps): JSX.Element {
  const summary = (): QuestionUseDetails["summary"] => props.usage.summary;
  return (
    <section class="catalog-statistics-panel catalog-usage-panel" aria-labelledby="usage-heading">
      <h2 id="usage-heading">Usage across PLE</h2>
      <p class="catalog-statistics-introduction">
        This exact published question appears in{" "}
        {formatCount(summary().globalCourseCount, "course")} and{" "}
        {formatCount(summary().globalAssignmentCount, "assignment")} across the Question Library.
        Course names below are limited to courses you can open.
      </p>
      <dl class="catalog-usage-counts">
        <div>
          <dt>Your courses</dt>
          <dd>{formatCount(summary().ownCourseCount, "course")}</dd>
        </div>
        <div>
          <dt>Your assignments</dt>
          <dd>{formatCount(summary().ownAssignmentCount, "assignment")}</dd>
        </div>
      </dl>
      <Show
        when={props.usage.ownCourses.length > 0}
        fallback={
          <p class="catalog-usage-next-step">
            <A href="/">Open your courses</A> to add this question to a future assignment.
          </p>
        }
      >
        <h3>Your courses using this question</h3>
        <ul class="catalog-usage-courses">
          <For each={props.usage.ownCourses}>
            {(course) => (
              <li>
                <A href={`/courses/${courseRouteReference(course.course)}`}>{course.title}</A>
                <span>{`${wholeNumber.format(course.assignmentCount)} assignment${course.assignmentCount === 1 ? "" : "s"}`}</span>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={props.usage.ownCoursesTruncated}>
        <p class="catalog-usage-next-step" role="status">
          More of your courses use this question. <A href="/">Open your courses</A> to continue the
          impact review.
        </p>
      </Show>
      <p class="catalog-usage-next-step">
        Review these course uses before replacing the question. A future assignment can use a
        replacement; issued student work remains unchanged.
      </p>
    </section>
  );
}
