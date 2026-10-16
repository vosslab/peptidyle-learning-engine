// question_statistics_panel.tsx - explainable, answer-free Question Statistics and use detail.

import { A } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { QuestionStatistics } from "../../generated/api/QuestionStatistics";
import type { QuestionUseDetails } from "../../generated/api/QuestionUseDetails";
import type { QuestionRevisionUsageStatistics } from "../../generated/api/QuestionRevisionUsageStatistics";
import { courseInstanceRouteId } from "../navigation/public_route";
import type { RecordContent } from "../components/record_list/record_list";
import type { RecordField } from "../components/record_list/record_fields";
import { SearchBlock } from "../features/search/search_block";
import { SearchControls } from "../features/search/search_controls";
import { createLocalCompleteSearch } from "../features/search/local_complete_search_state";
import "./question_statistics_panel.css";

export interface QuestionStatisticsPanelProps {
  readonly evidence: QuestionStatistics;
}

export interface QuestionUsePanelProps {
  readonly usage: QuestionUseDetails;
}

const wholeNumber = new Intl.NumberFormat("en-US");
const percentage = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 1,
});
type RevisionStatistics = QuestionRevisionUsageStatistics;

function revisionContent(revision: RevisionStatistics): RecordContent {
  return {
    title: `Revision ${revision.revision_number}`,
    details: [
      { kind: "text", label: "Received", value: wholeNumber.format(revision.issued_count) },
      {
        kind: "text",
        label: "Graded responses",
        value: wholeNumber.format(revision.answered_count),
      },
      {
        kind: "text",
        label: "Average stored credit",
        value: formatMean(revision.mean_credit, revision.answered_count),
      },
      {
        kind: "text",
        label: "Full credit",
        value: formatRate(revision.correct_rate, revision.answered_count),
      },
      {
        kind: "text",
        label: "Zero credit",
        value: formatRate(revision.incorrect_rate, revision.answered_count),
      },
    ],
    actions: [],
  };
}

type CourseUse = QuestionUseDetails["ownCourses"][number];

const revisionFields: ReadonlyArray<RecordField<RevisionStatistics, "revision", never>> = [
  {
    id: "revision",
    label: "Revision",
    role: "identity",
    value: (revision) => `Revision ${revision.revision_number}`,
    sortKey: "revision",
    defaultColumn: { priority: 1 },
  },
  {
    id: "received",
    label: "Received",
    value: (revision) => revision.issued_count,
    defaultColumn: { priority: 2 },
  },
  {
    id: "graded",
    label: "Graded responses",
    value: (revision) => revision.answered_count,
    defaultColumn: { priority: 3 },
  },
];

const courseUseFields: ReadonlyArray<RecordField<CourseUse, "title", never>> = [
  {
    id: "title",
    label: "Course",
    role: "identity",
    value: (course) => course.title,
    sortKey: "title",
    defaultColumn: { priority: 1 },
  },
  {
    id: "assessment_count",
    label: "Assessments",
    value: (course) => course.assessmentCount,
    defaultColumn: { priority: 2 },
  },
];

function courseUseContent(course: CourseUse): RecordContent {
  return {
    title: course.title,
    details: [
      {
        kind: "text",
        label: "Assessment count",
        value: formatCount(course.assessmentCount, "assessment"),
      },
    ],
    actions: [
      {
        id: "open-course",
        kind: "link",
        label: "Open Course",
        href: `/courses/${courseInstanceRouteId(course.courseInstanceId)}`,
        title: `Open Course: ${course.title}`,
      },
    ],
  };
}

function formatCount(value: number, singular: string): string {
  return `${wholeNumber.format(value)} ${value === 1 ? singular : `${singular}s`}`;
}

function formatRate(rate: number | undefined, observations: number): string {
  if (rate === undefined) {
    return `${wholeNumber.format(observations)} observations`;
  }
  return `${percentage.format(rate)} (${wholeNumber.format(observations)})`;
}

function formatMean(mean: number | undefined, observations: number): string {
  if (mean === undefined) {
    return `${wholeNumber.format(observations)} observations`;
  }
  return `${percentage.format(mean)} (${wholeNumber.format(observations)})`;
}

function RevisionStatisticsList(props: {
  readonly rows: readonly RevisionStatistics[];
}): JSX.Element {
  const [revisions, { refetch }] = createResource(() => props.rows);
  const search = createLocalCompleteSearch(
    { resource: revisions, refetchSource: refetch, rows: (value) => value },
    {
      fields: revisionFields,
      rowId: (revision) => String(revision.revision_number),
      keyword: (revision) => `Revision ${revision.revision_number}`,
      compare: (_key, left, right) => left.revision_number - right.revision_number,
      content: revisionContent,
    },
  );
  return (
    <SearchBlock
      layout="above"
      controls={<SearchControls state={search.state} textLabel="Search revisions" />}
      state={search.state}
      collection={{
        ariaLabel: "Question statistics by revision",
        emptyState: { title: "No revision statistics match your search." },
        fields: revisionFields,
        query: search.query,
      }}
    />
  );
}

function OwnCourseUseList(props: { readonly rows: readonly CourseUse[] }): JSX.Element {
  const [courses, { refetch }] = createResource(() => props.rows);
  const search = createLocalCompleteSearch(
    { resource: courses, refetchSource: refetch, rows: (value) => value },
    {
      fields: courseUseFields,
      rowId: (course) => course.courseInstanceId,
      keyword: (course) => course.title,
      content: courseUseContent,
    },
  );
  return (
    <SearchBlock
      layout="above"
      controls={<SearchControls state={search.state} textLabel="Search your courses" />}
      state={search.state}
      collection={{
        ariaLabel: "Your courses using this question",
        emptyState: { title: "No courses match your search." },
        fields: courseUseFields,
        query: search.query,
      }}
    />
  );
}

/** Renders privacy-safe Library statistics with their observation counts. */
export function QuestionStatisticsPanel(props: QuestionStatisticsPanelProps): JSX.Element {
  return (
    <Show
      when={props.evidence.state === "available" ? props.evidence : undefined}
      fallback={
        <section
          class="question-statistics-panel"
          aria-labelledby="question-statistics-unavailable-heading"
        >
          <h2 id="question-statistics-unavailable-heading">Statistics</h2>
          <p>
            Statistics are unavailable until enough Students have contributed to protect their
            privacy.
          </p>
        </section>
      }
    >
      {(available) => (
        <section class="question-statistics-panel" aria-labelledby="question-statistics-heading">
          <h2 id="question-statistics-heading">Statistics</h2>
          <p class="question-statistics-introduction">
            Received counts committed deliveries. Graded responses are those with stored credit.
            Average stored credit is the mean for graded responses, shown as a percentage.
            Full-credit and zero-credit percentages show the shares of graded responses. The count
            in parentheses is the number of graded responses used.
          </p>
          <dl class="question-statistics-measures">
            <div>
              <dt>Received</dt>
              <dd>
                {wholeNumber.format(available().pool_issued_count ?? available().issued_count)}
              </dd>
            </div>
            <div>
              <dt>Graded responses</dt>
              <dd>{wholeNumber.format(available().answered_count)}</dd>
            </div>
            <div>
              <dt>Average stored credit</dt>
              <dd>{formatMean(available().mean_credit, available().answered_count)}</dd>
            </div>
            <div>
              <dt>Full credit</dt>
              <dd>{formatRate(available().correct_rate, available().answered_count)}</dd>
            </div>
            <div>
              <dt>Zero credit</dt>
              <dd>{formatRate(available().incorrect_rate, available().answered_count)}</dd>
            </div>
          </dl>
          <Show when={available().revisions}>
            {(revisions) => (
              <>
                <h3>By revision</h3>
                <RevisionStatisticsList rows={revisions()} />
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
        {formatCount(summary().globalAssessmentCount, "assessment")}. Course names below are limited
        to courses you can open.
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
        <OwnCourseUseList rows={props.usage.ownCourses} />
      </Show>
      <p class="question-usage-next-step">
        Review these course uses before replacing the question. A future assessment can use a
        replacement; issued student work remains unchanged.
      </p>
    </section>
  );
}
