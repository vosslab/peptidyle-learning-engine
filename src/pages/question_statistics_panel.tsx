// question_statistics_panel.tsx - explainable, answer-free Question Statistics and use detail.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionUseDetails } from "../../generated/api/QuestionUseDetails";
import type { QuestionRevisionUsageStatistics } from "../../generated/api/QuestionRevisionUsageStatistics";
import { courseInstanceRouteId } from "../navigation/public_route";
import { RecordList } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
import "./question_statistics_panel.css";

export interface QuestionStatisticsPanelProps {
  readonly evidence: QuestionStatistics;
}

export interface QuestionUsePanelProps {
  readonly usage: QuestionUseDetails;
}

const wholeNumber = new Intl.NumberFormat("en-US");
type RevisionStatistics = QuestionRevisionUsageStatistics;

const revisionRegions: ReadonlyArray<RecordRegion<RevisionStatistics>> = [
  {
    id: "revision",
    role: "identity",
    priority: "required",
    width: "minmax(8rem, 1fr)",
    align: "start",
    content: (revision) => <>Revision {revision.revision_number}</>,
  },
  {
    id: "mean-credit",
    role: "metadata",
    priority: "required",
    width: "minmax(12rem, auto)",
    align: "end",
    content: (revision) => formatRate(revision.mean_credit, revision.answered_count),
  },
];

type CourseUse = QuestionUseDetails["ownCourses"][number];

const courseUseRegions: ReadonlyArray<RecordRegion<CourseUse>> = [
  {
    id: "course",
    role: "identity",
    priority: "required",
    width: "minmax(0, 1fr)",
    align: "start",
    content: (course) => (
      <A href={`/courses/${courseInstanceRouteId(course.courseInstanceId)}`}>{course.title}</A>
    ),
  },
  {
    id: "assessments",
    role: "metadata",
    priority: "required",
    width: "minmax(9rem, auto)",
    align: "end",
    content: (course) =>
      `${wholeNumber.format(course.assessmentCount)} assessment${course.assessmentCount === 1 ? "" : "s"}`,
  },
];

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
                <RecordList
                  ariaLabel="Question statistics by revision"
                  emptyState={{ title: "No revision statistics are available." }}
                  recordId={(revision) => String(revision.revision_number)}
                  regions={revisionRegions}
                  rows={revisions()}
                  state={{ kind: "ready" }}
                />
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
        <RecordList
          ariaLabel="Your courses using this question"
          emptyState={{ title: "No accessible courses use this question." }}
          recordId={(course) => course.courseInstanceId}
          regions={courseUseRegions}
          rows={props.usage.ownCourses}
          state={{ kind: "ready" }}
        />
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
