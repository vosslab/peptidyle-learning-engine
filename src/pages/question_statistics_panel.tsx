// question_statistics_panel.tsx - explainable, answer-free Question Statistics and use detail.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionUseDetails } from "../../generated/api/QuestionUseDetails";
import { courseInstanceRouteId } from "../navigation/public_route";
import "./question_statistics_panel.css";

export interface QuestionStatisticsPanelProps {
  readonly evidence: QuestionStatistics;
}

export interface QuestionUsePanelProps {
  readonly usage: QuestionUseDetails;
}

const wholeNumber = new Intl.NumberFormat("en-US");

function formatCount(value: number, singular: string): string {
  return `${wholeNumber.format(value)} ${value === 1 ? singular : `${singular}s`}`;
}

function formatRate(rate: number | undefined, observations: number): string {
  if (rate === undefined) {
    return `${wholeNumber.format(observations)} observations`;
  }
  return `${new Intl.NumberFormat("en-US", { style: "percent", maximumFractionDigits: 1 }).format(rate)} (${wholeNumber.format(observations)})`;
}

function formatMean(mean: number | undefined, observations: number): string {
  if (mean === undefined) {
    return `${wholeNumber.format(observations)} observations`;
  }
  return `${mean.toFixed(2)} (${wholeNumber.format(observations)})`;
}

/** Renders Instructor-visible usage rates beside their observation counts. */
export function QuestionStatisticsPanel(props: QuestionStatisticsPanelProps): JSX.Element {
  return (
    <Show
      when={props.evidence.state === "available" ? props.evidence : undefined}
      fallback={
        <section
          class="question-statistics-panel"
          aria-labelledby="question-statistics-unavailable-heading"
        >
          <h2 id="question-statistics-unavailable-heading">Learning evidence</h2>
          <p>
            Question Statistics are unavailable until shared learning measures can be shown. This
            question remains ranked by relevance, so you can still open it and decide whether it
            fits.
          </p>
        </section>
      }
    >
      {(available) => (
        <section class="question-statistics-panel" aria-labelledby="question-statistics-heading">
          <h2 id="question-statistics-heading">Question usage</h2>
          <p class="question-statistics-introduction">
            Global, identity-free counts for this Question. Each rate is shown with the number of
            observations used as its denominator.
          </p>
          <dl class="question-statistics-measures">
            <div>
              <dt>Blank rate</dt>
              <dd>{formatRate(available().blank_rate, available().issued_count)}</dd>
            </div>
            <div>
              <dt>Answered rate</dt>
              <dd>{formatRate(available().answered_rate, available().issued_count)}</dd>
            </div>
            <div>
              <dt>Correct rate</dt>
              <dd>{formatRate(available().correct_rate, available().answered_count)}</dd>
            </div>
            <div>
              <dt>Partial rate</dt>
              <dd>{formatRate(available().partial_rate, available().answered_count)}</dd>
            </div>
            <div>
              <dt>Incorrect rate</dt>
              <dd>{formatRate(available().incorrect_rate, available().answered_count)}</dd>
            </div>
            <div>
              <dt>Mean credit</dt>
              <dd>{formatMean(available().mean_credit, available().answered_count)}</dd>
            </div>
          </dl>
          <Show when={available().revisions}>
            {(revisions) => (
              <>
                <h3>By revision</h3>
                <For each={revisions()}>
                  {(revision) => (
                    <p>
                      Revision {revision.revision_number}:{" "}
                      {formatRate(revision.mean_credit, revision.answered_count)}
                    </p>
                  )}
                </For>
              </>
            )}
          </Show>
        </section>
      )}
    </Show>
  );
}

/** Shows installation-wide counts and Account-authorized course links. */
export function QuestionUsePanel(props: QuestionUsePanelProps): JSX.Element {
  const summary = (): QuestionUseDetails["summary"] => props.usage.summary;
  return (
    <section class="question-statistics-panel question-usage-panel" aria-labelledby="usage-heading">
      <h2 id="usage-heading">Usage across PLE</h2>
      <p class="question-statistics-introduction">
        This exact published question appears in{" "}
        {formatCount(summary().globalCourseCount, "course")} and{" "}
        {formatCount(summary().globalAssessmentCount, "assessment")} across the Question Library.
        Course names below are limited to courses you can open.
      </p>
      <dl class="question-usage-counts">
        <div>
          <dt>Your courses</dt>
          <dd>{formatCount(summary().ownCourseCount, "course")}</dd>
        </div>
        <div>
          <dt>Your assessments</dt>
          <dd>{formatCount(summary().ownAssessmentCount, "assessment")}</dd>
        </div>
      </dl>
      <Show
        when={props.usage.ownCourses.length > 0}
        fallback={
          <p class="question-usage-next-step">
            <A href="/">Open your courses</A> to add this question to a future assessment.
          </p>
        }
      >
        <h3>Your courses using this question</h3>
        <ul class="question-usage-courses">
          <For each={props.usage.ownCourses}>
            {(course) => (
              <li>
                <A href={`/courses/${courseInstanceRouteId(course.course)}`}>{course.title}</A>
                <span>{`${wholeNumber.format(course.assessmentCount)} assessment${course.assessmentCount === 1 ? "" : "s"}`}</span>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={props.usage.ownCoursesTruncated}>
        <p class="question-usage-next-step" role="status">
          More of your courses use this question. <A href="/">Open your courses</A> to continue the
          impact review.
        </p>
      </Show>
      <p class="question-usage-next-step">
        Review these course uses before replacing the question. A future assessment can use a
        replacement; issued student work remains unchanged.
      </p>
    </section>
  );
}
