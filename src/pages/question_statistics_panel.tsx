// question_statistics_panel.tsx - explainable, answer-free Question Statistics and use detail.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionUseDetails } from "../../generated/api/QuestionUseDetails";
import { courseInstanceRouteReference } from "../navigation/public_route";
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

/** Renders the explicit unavailable state until a release service exists. */
export function QuestionStatisticsPanel(_props: QuestionStatisticsPanelProps): JSX.Element {
  return (
    <section
      class="question-statistics-panel"
      aria-labelledby="question-statistics-unavailable-heading"
    >
      <h2 id="question-statistics-unavailable-heading">Learning evidence</h2>
      <p>
        Question Statistics are unavailable until shared learning measures can be shown. This
        question remains ranked by relevance, so you can still open it and decide whether it fits.
      </p>
    </section>
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
        {formatCount(summary().globalAssignmentCount, "assignment")} across the Question Library.
        Course names below are limited to courses you can open.
      </p>
      <dl class="question-usage-counts">
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
          <p class="question-usage-next-step">
            <A href="/">Open your courses</A> to add this question to a future assignment.
          </p>
        }
      >
        <h3>Your courses using this question</h3>
        <ul class="question-usage-courses">
          <For each={props.usage.ownCourses}>
            {(course) => (
              <li>
                <A href={`/courses/${courseInstanceRouteReference(course.course)}`}>
                  {course.title}
                </A>
                <span>{`${wholeNumber.format(course.assignmentCount)} assignment${course.assignmentCount === 1 ? "" : "s"}`}</span>
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
        Review these course uses before replacing the question. A future assignment can use a
        replacement; issued student work remains unchanged.
      </p>
    </section>
  );
}
